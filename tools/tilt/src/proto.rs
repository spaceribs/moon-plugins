use extism_pdk::*;
use proto_pdk::*;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

const NAME: &str = "Tilt";
const REPO: &str = "https://github.com/tilt-dev/tilt";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 50, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags(REPO)?
        .into_iter()
        .filter_map(|tag| tag.strip_prefix('v').map(|tag| tag.to_owned()))
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        NAME,
        env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64, HostArch::Arm],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let version = input.context.version;

    let os = match env.os {
        // Alpine builds are statically linked against musl
        HostOS::Linux if env.libc == HostLibc::Musl => {
            if env.arch == HostArch::Arm {
                return Err(plugin_err!(PluginError::UnsupportedTarget {
                    tool: NAME.into(),
                    arch: "arm (musl)".into(),
                    os: env.os.to_string(),
                }));
            }

            "linux-alpine"
        }
        HostOS::Linux => "linux",
        HostOS::MacOS => "mac",
        HostOS::Windows => "windows",
        _ => unreachable!(),
    };

    let arch = match env.arch {
        HostArch::X64 => "x86_64",
        HostArch::Arm64 => "arm64",
        HostArch::Arm => "arm",
        _ => unreachable!(),
    };

    let ext = if env.os.is_windows() { "zip" } else { "tar.gz" };
    let download_file = format!("tilt.{version}.{os}.{arch}.{ext}");
    let base_url = format!("{REPO}/releases/download/v{version}");

    Ok(Json(DownloadPrebuiltOutput {
        checksum_url: Some(format!("{base_url}/checksums.txt")),
        download_url: format!("{base_url}/{download_file}"),
        download_name: Some(download_file),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: [(
            "tilt".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name("tilt")),
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    }))
}
