//! Finds the Tilt binary that the Tilt toolchain installed through proto, so
//! the extension runs the pinned version directly. The plugin host resolves
//! bare commands on the user's `PATH`, and proto's `tilt` shim only works when
//! `.prototools` registers a `tilt` plugin, so neither can be relied on.

use serde::Deserialize;
use version_spec::{MatchesVersion, UnresolvedVersionSpec, Version};

/// Where proto installs the toolchain's Tilt versions (`/proto` is
/// `PROTO_HOME`), each at `<TOOL_DIR>/<version>/tilt`.
pub const TOOL_DIR: &str = "/proto/tools/tilt";

/// proto's home directory, as mounted into the plugin.
pub const PROTO_MOUNT: &str = "/proto";

/// `path`, a path under [`PROTO_MOUNT`], on the host, where proto's home is
/// `proto_home`. The host runs commands outside the plugin's sandbox, so it
/// can't resolve virtual paths itself.
pub fn to_host_path(path: &str, proto_home: &str, windows: bool) -> String {
    let Some(rest) = path
        .strip_prefix(PROTO_MOUNT)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
    else {
        return path.to_owned();
    };
    let home = proto_home.trim_end_matches(['/', '\\']);

    if windows {
        format!("{home}{}", rest.replace('/', "\\"))
    } else {
        format!("{home}{rest}")
    }
}

/// No installed version satisfies the toolchain's version.
#[derive(Debug, PartialEq)]
pub struct NotInstalled {
    pub requested: String,
    pub installed: Vec<String>,
}

impl NotInstalled {
    pub fn message(&self) -> String {
        let installed = if self.installed.is_empty() {
            "none".to_owned()
        } else {
            self.installed.join(", ")
        };

        format!(
            "Tilt <hash>{}</hash> (pinned by the Tilt toolchain) isn't installed. Installed versions: {installed}. \
             Run <shell>moon setup</shell> to install it.",
            self.requested
        )
    }
}

/// The Tilt command to run, and a directory to put first on its `PATH`.
#[derive(Debug, PartialEq)]
pub struct TiltExecutable {
    pub command: String,
    pub path: Option<String>,
}

/// What runs Tilt: an explicit `tiltBin` other than `tilt` as-is; otherwise,
/// with the toolchain pinning `toolchain_version`, the installed binary of the
/// highest matching version (with its directory first on `PATH`, for anything
/// Tilt starts); otherwise `tilt` from `PATH`. `manifest` is proto's
/// `manifest.json` for Tilt, if readable, and `exe` the binary's file name in
/// an install ([`exe_name`]).
pub fn tilt_executable(
    tilt_bin: &str,
    toolchain_version: Option<&str>,
    manifest: Option<&str>,
    exe: &str,
) -> Result<TiltExecutable, NotInstalled> {
    let plain = |command: &str| TiltExecutable {
        command: command.to_owned(),
        path: None,
    };

    if tilt_bin != "tilt" {
        return Ok(plain(tilt_bin));
    }

    let Some(requested) = toolchain_version else {
        return Ok(plain("tilt"));
    };
    let version = resolve_installed(requested, manifest)?;

    Ok(TiltExecutable {
        command: format!("{TOOL_DIR}/{version}/{exe}"),
        path: Some(format!("{TOOL_DIR}/{version}")),
    })
}

/// The Tilt binary's file name in a proto install on the host OS.
pub fn exe_name(windows: bool) -> &'static str {
    if windows { "tilt.exe" } else { "tilt" }
}

/// proto's per-tool `manifest.json`.
#[derive(Default, Deserialize)]
#[serde(default)]
struct Manifest {
    installed_versions: Vec<String>,
}

/// The highest installed version (from proto's tool manifest) that satisfies
/// `requested`: an exact version, a partial version or requirement (`0.37`,
/// `~0.37`), a range, or an alias (`latest`, `stable`), which resolves to the
/// highest installed version. A missing or malformed manifest means nothing
/// is installed.
pub fn resolve_installed(requested: &str, manifest: Option<&str>) -> Result<String, NotInstalled> {
    let installed: Vec<String> = manifest
        .and_then(|manifest| serde_json::from_str::<Manifest>(manifest).ok())
        .unwrap_or_default()
        .installed_versions;
    let not_installed = || NotInstalled {
        requested: requested.to_owned(),
        installed: installed.clone(),
    };
    let spec = UnresolvedVersionSpec::parse(requested).map_err(|_| not_installed())?;
    let matches = |version: &Version| match &spec {
        UnresolvedVersionSpec::Canary => false,
        UnresolvedVersionSpec::Alias(_) => version.prerelease.is_none(),
        UnresolvedVersionSpec::Range(range) => range.matches(version),
        UnresolvedVersionSpec::Requirement(requirement) => requirement.matches(version),
        UnresolvedVersionSpec::Version(exact) => exact.matches(version),
    };

    installed
        .iter()
        .filter_map(|raw| Some((Version::parse(raw).ok()?, raw)))
        .filter(|(version, _)| matches(version))
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, raw)| raw.clone())
        .ok_or_else(not_installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "installed_versions": ["0.36.3", "0.37.7", "0.37.8", "0.38.0-rc.1"],
        "shim_version": 0,
        "versions": {}
    }"#;

    fn resolve(requested: &str) -> Result<String, NotInstalled> {
        resolve_installed(requested, Some(MANIFEST))
    }

    #[test]
    fn exact_version() {
        assert_eq!(resolve("0.37.7").unwrap(), "0.37.7");
    }

    #[test]
    fn partial_versions_and_requirements_pick_the_highest_match() {
        assert_eq!(resolve("0.37").unwrap(), "0.37.8");
        assert_eq!(resolve("~0.37").unwrap(), "0.37.8");
        assert_eq!(resolve("<0.37").unwrap(), "0.36.3");
    }

    #[test]
    fn ranges() {
        assert_eq!(resolve(">=0.36, <0.37.8").unwrap(), "0.37.7");
        assert_eq!(resolve("0.36 || 0.37").unwrap(), "0.37.8");
    }

    #[test]
    fn aliases_pick_the_highest_stable_version() {
        assert_eq!(resolve("latest").unwrap(), "0.37.8");
        assert_eq!(resolve("stable").unwrap(), "0.37.8");
    }

    #[test]
    fn reports_what_is_installed_when_nothing_matches() {
        let error = resolve("0.35.0").unwrap_err();

        assert_eq!(
            error.installed,
            ["0.36.3", "0.37.7", "0.37.8", "0.38.0-rc.1"]
        );
        assert!(error.message().contains("Tilt <hash>0.35.0</hash>"));
        assert!(
            error
                .message()
                .contains("Installed versions: 0.36.3, 0.37.7")
        );
        assert!(error.message().contains("<shell>moon setup</shell>"));
    }

    #[test]
    fn runs_the_installed_binary_of_the_pinned_version() {
        assert_eq!(
            tilt_executable("tilt", Some("0.37"), Some(MANIFEST), exe_name(false)).unwrap(),
            TiltExecutable {
                command: "/proto/tools/tilt/0.37.8/tilt".into(),
                path: Some("/proto/tools/tilt/0.37.8".into()),
            }
        );
    }

    #[test]
    fn runs_tilt_exe_on_windows() {
        assert_eq!(
            tilt_executable("tilt", Some("0.37.8"), Some(MANIFEST), exe_name(true)).unwrap(),
            TiltExecutable {
                command: "/proto/tools/tilt/0.37.8/tilt.exe".into(),
                path: Some("/proto/tools/tilt/0.37.8".into()),
            }
        );
        // Falling back to PATH stays `tilt`: Windows resolves the extension.
        assert_eq!(
            tilt_executable("tilt", None, None, exe_name(true))
                .unwrap()
                .command,
            "tilt"
        );
    }

    #[test]
    fn falls_back_to_path_without_a_pinned_version() {
        assert_eq!(
            tilt_executable("tilt", None, None, exe_name(false)).unwrap(),
            TiltExecutable {
                command: "tilt".into(),
                path: None,
            }
        );
    }

    #[test]
    fn a_custom_tilt_bin_takes_precedence() {
        // Even when the pinned version isn't installed.
        assert_eq!(
            tilt_executable("/opt/tilt/bin/tilt", Some("0.37.8"), None, exe_name(false)).unwrap(),
            TiltExecutable {
                command: "/opt/tilt/bin/tilt".into(),
                path: None,
            }
        );
    }

    #[test]
    fn fails_when_the_pinned_version_is_not_installed() {
        let error =
            tilt_executable("tilt", Some("0.35"), Some(MANIFEST), exe_name(false)).unwrap_err();

        assert_eq!(error.requested, "0.35");
    }

    #[test]
    fn canary_is_never_installed() {
        assert!(resolve("canary").is_err());
    }

    #[test]
    fn missing_or_malformed_manifest_means_nothing_installed() {
        for manifest in [None, Some("not json"), Some("{}")] {
            let error = resolve_installed("0.37.8", manifest).unwrap_err();

            assert!(error.installed.is_empty());
            assert!(error.message().contains("Installed versions: none."));
        }
    }

    #[test]
    fn maps_proto_paths_to_the_host() {
        assert_eq!(
            to_host_path("/proto/tools/tilt/0.37.8/tilt", "/home/me/.proto", false),
            "/home/me/.proto/tools/tilt/0.37.8/tilt"
        );
        assert_eq!(
            to_host_path("/proto/tools/tilt/0.37.8", "/home/me/.proto/", false),
            "/home/me/.proto/tools/tilt/0.37.8"
        );
        assert_eq!(
            to_host_path(
                "/proto/tools/tilt/0.37.8/tilt.exe",
                "C:\\Users\\me\\.proto",
                true
            ),
            "C:\\Users\\me\\.proto\\tools\\tilt\\0.37.8\\tilt.exe"
        );
    }

    #[test]
    fn leaves_other_paths_alone() {
        assert_eq!(to_host_path("tilt", "/home/me/.proto", false), "tilt");
        assert_eq!(
            to_host_path("/protocol/tilt", "/home/me/.proto", false),
            "/protocol/tilt"
        );
    }
}
