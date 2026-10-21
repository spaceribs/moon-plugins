use serde::{Deserialize, Serialize};

/// When Tilt restarts a service (`serve_cmd`) resource.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, schematic::Schematic)]
#[serde(rename_all = "camelCase")]
pub enum ServiceRestart {
    /// Restart when moon rebuilds an upstream task dependency, i.e. when its
    /// outputs change. Source edits are left to the service's own live reload
    /// (webpack, vite, nodemon, ...), so hot-reloading servers keep their state.
    #[default]
    UpstreamOutputs,
    /// Restart when the service task's own inputs change. For servers without
    /// live reload and without a build task.
    Inputs,
    /// Never restart on file changes.
    Never,
}

/// The label the extension adds to declared workloads (`project.tilt.workloads`).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, schematic::Schematic)]
#[serde(rename_all = "camelCase")]
pub enum WorkloadLabels {
    /// Label each workload with its project ID, like local resources.
    #[default]
    Project,
    /// Add no label, leaving the labels set by the project's own Tiltfile.
    None,
}

/// Settings for the Tilt extension, configured in `.moon/extensions.yml`.
///
/// ```yaml
/// tilt:
///   plugin: "github://spaceribs/moon-plugins/tilt_extension"
///   tag: "tilt"
///   outputDir: ".moon/tilt"
/// ```
#[derive(Clone, Debug, Deserialize, schematic::Schematic)]
#[serde(default, rename_all = "camelCase")]
pub struct TiltExtensionConfig {
    /// Tasks (or projects) with this tag are exposed as Tilt resources.
    /// Tagged projects contribute all of their persistent tasks.
    pub tag: String,

    /// Expose every persistent (`preset: server`) task as a resource,
    /// regardless of tags.
    pub include_persistent: bool,

    /// Make services wait on the services of the projects they depend on
    /// (moon's project graph), following through projects without services.
    pub link_project_dependencies: bool,

    /// Also expose upstream services that aren't tagged, so every service a
    /// selected service depends on is represented in Tilt.
    pub include_upstream_services: bool,

    /// Project dependency scopes that represent runtime relationships.
    pub dependency_scopes: Vec<String>,

    /// `include()` every project's own `Tiltfile` from the generated file, so
    /// the main Tiltfile brings up the entire cluster.
    pub include_project_tiltfiles: bool,

    /// Regenerate during moon's workspace sync (`moon sync` and before
    /// `moon run`), instead of only through `moon ext tilt -- generate`.
    pub sync: bool,

    /// Apply the workspace's gitignore patterns to every Tilt resource
    /// (`watch_settings`), including ones in your own Tiltfiles, so Tilt
    /// neither watches nor rebuilds for gitignored files.
    pub global_gitignore: bool,

    /// Regenerate the generated file from a `(moon)` Tilt resource when moon's
    /// configuration changes while Tilt is running.
    pub watch_config: bool,

    /// Build container images from moon's generated Dockerfiles for projects
    /// that opt in with `project.tilt.image`.
    pub images: bool,

    /// Binary used to build images.
    pub docker_bin: String,

    /// Sync source changes into running containers (Tilt Live Update)
    /// instead of rebuilding images. Override per project with
    /// `project.tilt.liveUpdate`.
    pub live_update: bool,

    /// Working directory of moon's generated images (`WORKDIR`), where the
    /// workspace layout is reproduced and sources are synced to.
    pub image_workdir: String,

    /// Accept moon project IDs as Tilt args (`tilt up -- web`, `tilt args --
    /// web`) through `config.parse()`. Disable if your own Tiltfile already
    /// defines Tiltfile args.
    pub select_by_project: bool,

    /// When to restart services. Override per project with
    /// `project.metadata.tilt.serviceRestart` in moon.yml.
    pub service_restart: ServiceRestart,

    /// The label added to the workloads projects declare in
    /// `project.tilt.workloads`: `project` (their project ID) or `none`, for
    /// Tiltfiles that already label their workloads.
    pub workload_labels: WorkloadLabels,

    /// Workspace-relative directory of section templates. A `<section>.tera`
    /// there (`Tiltfile`, `lib`, `args`, `resources`, `image`, `workloads`,
    /// `selection`) replaces that section's built-in template, and can
    /// `{% extends "moon/sections/<section>.tera" %}` to override blocks.
    pub template_dir: Option<String>,

    /// Removed: use `templateDir`. Setting it fails generation.
    pub template: Option<String>,

    /// Workspace-relative directory the generated Tiltfiles are written to.
    /// It ignores itself (`.gitignore`), and the main Tiltfile includes its
    /// `Tiltfile`.
    pub output_dir: String,

    /// Removed: use `outputDir`. Setting it fails generation.
    pub output: Option<String>,

    /// Workspace-relative path of the main Tiltfile. Created when it does not
    /// exist, regenerating the output and then including it.
    pub tiltfile: String,

    /// Watch task inputs with Tilt and re-run/restart resources on change.
    pub watch_inputs: bool,

    /// Binary used to invoke moon from Tilt resources.
    pub moon_bin: String,

    /// Binary used to invoke Tilt. With the default (`tilt`) and the Tilt
    /// toolchain configured, the binary the toolchain installed for its pinned
    /// version is used instead. Any other value is used as-is.
    pub tilt_bin: String,
}

impl Default for TiltExtensionConfig {
    fn default() -> Self {
        Self {
            tag: "tilt".into(),
            include_persistent: false,
            link_project_dependencies: true,
            include_upstream_services: true,
            dependency_scopes: vec!["production".into(), "development".into(), "peer".into()],
            include_project_tiltfiles: true,
            sync: true,
            global_gitignore: true,
            watch_config: true,
            images: true,
            docker_bin: "docker".into(),
            live_update: true,
            image_workdir: "/app".into(),
            select_by_project: true,
            service_restart: ServiceRestart::default(),
            workload_labels: WorkloadLabels::default(),
            template_dir: None,
            template: None,
            output_dir: ".moon/tilt".into(),
            output: None,
            tiltfile: "Tiltfile".into(),
            watch_inputs: true,
            moon_bin: "moon".into(),
            tilt_bin: "tilt".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> TiltExtensionConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn workload_labels_default_to_project() {
        assert_eq!(parse("{}").workload_labels, WorkloadLabels::Project);
    }

    #[test]
    fn parses_workload_labels() {
        assert_eq!(
            parse(r#"{ "workloadLabels": "project" }"#).workload_labels,
            WorkloadLabels::Project
        );
        assert_eq!(
            parse(r#"{ "workloadLabels": "none" }"#).workload_labels,
            WorkloadLabels::None
        );
    }
}
