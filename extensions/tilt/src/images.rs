//! Container images built from moon's generated Dockerfiles, for projects that
//! opt in with `project.tilt.image`. The user's own Kubernetes or Compose
//! resources deploy them by referencing the image name.
//!
//! moon's Dockerfiles copy the whole workspace and run `moon docker scaffold`,
//! so the build context must stay the workspace root, and `docker_build(only=)`
//! (which also shrinks the context) can't be used. `custom_build` decouples the
//! two: the command builds with the full context, while `deps` (what triggers
//! a rebuild) comes from moon's project graph, like Tilt's Bazel integration.

use crate::config::TiltExtensionConfig;
use crate::query::{QueryProjectsOutput, QueryTasksOutput};
use crate::tiltfile::path_to_root;
use crate::watch::{expand_braces, glob_base};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Where generated Dockerfiles are written (gitignored moon cache).
pub const DOCKERFILE_DIR: &str = ".moon/cache/tilt";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Image {
    /// moon project the image is built for.
    pub project: String,
    /// Image name to reference in k8s YAML or compose files, used verbatim as
    /// the `custom_build` ref (registry prefixes included).
    pub image_ref: String,
    /// Workspace-relative path of the generated Dockerfile.
    pub dockerfile: String,
    /// Shell command that generates the Dockerfile and builds `$EXPECTED_REF`.
    pub command: String,
    /// Workspace-relative paths whose changes trigger a rebuild.
    pub watch: Vec<String>,
    /// Patterns within `watch` that don't trigger a rebuild (task outputs).
    pub ignore: Vec<String>,
    /// Sync source changes into running containers instead of rebuilding.
    pub live_update: Option<LiveUpdate>,
}

/// Tilt Live Update steps for an image: changed source files are synced into
/// the running container instead of rebuilding and redeploying the image.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LiveUpdate {
    /// Files whose changes need a full image rebuild (dependency manifests,
    /// lockfiles, moon config), checked before syncing.
    pub fall_back_on: Vec<String>,
    /// Workspace-relative project directories, synced to the same path under
    /// the image's working directory.
    pub syncs: Vec<Sync>,
    /// Restart the process after syncing (for apps without live reload),
    /// running this entrypoint through Tilt's `restart_process` extension.
    pub restart_entrypoint: Option<Vec<String>>,
    /// Restart the container after syncing instead (`restart_container()`),
    /// for images deployed with Docker Compose, where Tilt doesn't support
    /// `restart_process`'s entrypoint override. The container re-runs its CMD.
    pub restart_container: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Sync {
    pub local: String,
    pub remote: String,
}

/// Plan an image for every project that opts in with `project.tilt.image`.
/// `config_files` are the workspace-relative moon config files (the
/// Dockerfile is generated from them).
pub fn plan_images(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
    config_files: &[String],
) -> Vec<Image> {
    if !config.images {
        return vec![];
    }

    let by_id: BTreeMap<&str, &crate::query::QueryProject> = projects
        .projects
        .iter()
        .map(|project| (project.id.as_str(), project))
        .collect();

    projects
        .projects
        .iter()
        .filter_map(|project| {
            // Opt-in, so `docker.file` settings meant for production images
            // don't turn into local builds.
            let image_ref = project
                .config
                .tilt_metadata()
                .and_then(|tilt| tilt.get("image"))
                .and_then(|value| value.as_str())?
                .to_owned();

            let closure = dependency_closure(&project.id, &by_id);
            let (watch, ignore) = image_triggers(&closure, &by_id, tasks, config_files);
            let dockerfile = format!("{DOCKERFILE_DIR}/{}.Dockerfile", project.id);

            // `moon docker file` resolves its destination from the project root.
            let dest = match project.source.trim_matches('/') {
                "" | "." => dockerfile.clone(),
                source => format!("{}/{dockerfile}", path_to_root(&format!("{source}/_"))),
            };

            let live_update = plan_live_update(project, &closure, &by_id, tasks, config, config_files);

            Some(Image {
                project: project.id.clone(),
                image_ref,
                live_update,
                command: format!(
                    "{moon} docker file {id} {dest} --defaults && {docker} build -t $EXPECTED_REF -f {dockerfile} .",
                    moon = config.moon_bin,
                    id = project.id,
                    docker = config.docker_bin,
                ),
                dockerfile,
                watch,
                ignore,
            })
        })
        .collect()
}

/// Live Update for a project's image: sync the source directories of the
/// project and its dependencies (moon's Dockerfiles keep the workspace layout
/// under `WORKDIR /app`), and fall back to a full rebuild for files that need
/// one. Apps without live reload (`serviceRestart: inputs`) are restarted.
fn plan_live_update(
    project: &crate::query::QueryProject,
    closure: &BTreeSet<&str>,
    by_id: &BTreeMap<&str, &crate::query::QueryProject>,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
    config_files: &[String],
) -> Option<LiveUpdate> {
    let tilt = project.config.tilt_metadata();
    let enabled = tilt
        .and_then(|tilt| tilt.get("liveUpdate"))
        .and_then(|value| value.as_bool())
        .unwrap_or(config.live_update);

    if !enabled {
        return None;
    }

    let mut syncs = vec![];
    let mut fall_back_on = BTreeSet::new();

    for id in closure {
        let source = by_id
            .get(id)
            .map(|project| project.source.trim_matches('/'))
            .unwrap_or_default();
        let is_root = source.is_empty() || source == ".";

        if !is_root {
            syncs.push(Sync {
                local: source.to_owned(),
                remote: format!("{}/{source}", config.image_workdir.trim_end_matches('/')),
            });
        }

        // moon config, root-level inputs (lockfiles), and explicit task input
        // files (manifests like package.json) change dependencies or the
        // Dockerfile itself, which a sync can't apply.
        fall_back_on.extend(
            config_files
                .iter()
                .filter(|file| belongs_to(file, source, is_root))
                .cloned(),
        );

        for task in tasks
            .tasks
            .get(*id)
            .into_iter()
            .flat_map(|tasks| tasks.values())
        {
            fall_back_on.extend(
                task.input_files
                    .keys()
                    .filter(|file| !file.starts_with(".moon"))
                    .cloned(),
            );
        }
    }

    if syncs.is_empty() {
        return None;
    }

    syncs.sort_by(|a, b| a.local.cmp(&b.local));

    let restart = tilt
        .and_then(|tilt| tilt.get("serviceRestart"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or(config.service_restart);
    let start_task = project
        .config
        .docker
        .file
        .get("startTask")
        .and_then(|task| task.as_str());

    Some(LiveUpdate {
        fall_back_on: fall_back_on.into_iter().collect(),
        syncs,
        restart_entrypoint: match (restart, start_task) {
            (crate::config::ServiceRestart::Inputs, Some(task)) => Some(vec![
                "moon".into(),
                "run".into(),
                format!("{}:{task}", project.id),
            ]),
            _ => None,
        },
        restart_container: false,
    })
}

/// Whether a moon config file affects a project's image. Extension config
/// (like this one's) doesn't change what's built.
fn belongs_to(file: &str, source: &str, is_root: bool) -> bool {
    (file.starts_with(".moon/") && !file.starts_with(".moon/extensions."))
        || (!is_root && file.starts_with(&format!("{source}/moon.")))
        || (is_root && file.starts_with("moon."))
}

/// The project and every project it depends on, transitively (all scopes:
/// build-time dependencies end up in the image too).
fn dependency_closure<'a>(
    project_id: &'a str,
    by_id: &BTreeMap<&'a str, &'a crate::query::QueryProject>,
) -> BTreeSet<&'a str> {
    let mut closure = BTreeSet::new();
    let mut stack = vec![project_id];

    while let Some(id) = stack.pop() {
        if !closure.insert(id) {
            continue;
        }

        if let Some(project) = by_id.get(id) {
            stack.extend(
                project
                    .dependencies
                    .iter()
                    .flatten()
                    .map(|dep| dep.id.as_str()),
            );
        }
    }

    closure
}

/// What triggers a rebuild: the closure's project directories, moon config
/// for them (the Dockerfile is generated from it), and root-level files their
/// tasks take as inputs (lockfiles, shared configs). Task outputs are ignored:
/// moon's Dockerfiles build inside the container, not from host artifacts.
fn image_triggers(
    closure: &BTreeSet<&str>,
    by_id: &BTreeMap<&str, &crate::query::QueryProject>,
    tasks: &QueryTasksOutput,
    config_files: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut watch = BTreeSet::new();
    let mut ignore = BTreeSet::new();

    for id in closure {
        let source = by_id
            .get(id)
            .map(|project| project.source.trim_matches('/'))
            .unwrap_or_default();
        let is_root = source.is_empty() || source == ".";

        // Watching the workspace root project's directory would watch everything.
        if !is_root {
            watch.insert(source.to_owned());
        }

        for file in config_files {
            if belongs_to(file, source, is_root) {
                watch.insert(file.clone());
            }
        }

        for task in tasks
            .tasks
            .get(*id)
            .into_iter()
            .flat_map(|tasks| tasks.values())
        {
            for file in task.input_files.keys() {
                if !file.contains('/') && !file.starts_with(".moon") {
                    watch.insert(file.clone());
                }
            }

            for file in task.output_files.keys() {
                ignore.insert(file.clone());
                ignore.insert(format!("{file}/**"));
            }

            for glob in task.output_globs.keys() {
                if !glob_base(glob).is_empty() {
                    ignore.extend(expand_braces(glob));
                }
            }
        }
    }

    // Drop paths already covered by a watched directory.
    let dirs: Vec<String> = watch.iter().cloned().collect();
    let watch = dirs
        .iter()
        .filter(|path| {
            !dirs
                .iter()
                .any(|parent| parent != *path && path.starts_with(&format!("{parent}/")))
        })
        .cloned()
        .collect();

    (watch, ignore.into_iter().collect())
}

/// Tiltfile args: projects to run (positional) and projects whose images
/// build from source (`--to-edit`, absent = all, `none` = none).
#[derive(Debug, Default, PartialEq)]
pub struct TiltArgs {
    pub to_run: Vec<String>,
    pub to_edit: Option<Vec<String>>,
}

impl TiltArgs {
    pub fn parse(args: &[String]) -> Self {
        let mut parsed = TiltArgs::default();
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            if let Some(value) = arg.strip_prefix("--to-edit=") {
                parsed
                    .to_edit
                    .get_or_insert_with(Vec::new)
                    .push(value.to_owned());
            } else if arg == "--to-edit" {
                if let Some(value) = iter.next() {
                    parsed
                        .to_edit
                        .get_or_insert_with(Vec::new)
                        .push(value.clone());
                }
            } else {
                parsed.to_run.push(arg.clone());
            }
        }

        parsed
    }

    pub fn to_args(&self) -> Vec<String> {
        let mut args = self.to_run.clone();

        for project in self.to_edit.iter().flatten() {
            args.push("--to-edit".into());
            args.push(project.clone());
        }

        args
    }
}

/// Which images to build from source: explicit projects, plus affected ones.
/// `None` means all; an empty result becomes `none` (all prebuilt).
pub fn resolve_to_edit(explicit: &[String], affected: Option<Vec<String>>) -> Option<Vec<String>> {
    match affected {
        None if explicit.is_empty() => None,
        None => Some(explicit.to_vec()),
        Some(affected) => {
            let mut projects: BTreeSet<String> = explicit.iter().cloned().collect();
            projects.extend(affected);

            if projects.is_empty() {
                Some(vec!["none".into()])
            } else {
                Some(projects.into_iter().collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueryProjectMetadata;

    fn fixture() -> (QueryProjectsOutput, QueryTasksOutput) {
        let mut projects: QueryProjectsOutput =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/projects.json"))
                .unwrap();
        let tasks =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/tasks.json")).unwrap();

        // web and api build images (api under a registry-prefixed name),
        // which the user's own manifests deploy.
        let web = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "web")
            .unwrap();
        web.config
            .docker
            .file
            .insert("startTask".into(), "dev".into());
        web.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(r#"{ "tilt": { "image": "web" } }"#).unwrap(),
        });

        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();
        api.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(r#"{ "tilt": { "image": "acme/api" } }"#).unwrap(),
        });

        (projects, tasks)
    }

    fn config_files() -> Vec<String> {
        [
            ".moon/workspace.yml",
            "api/moon.yml",
            "db/moon.yml",
            "web/moon.yml",
        ]
        .map(String::from)
        .to_vec()
    }

    #[test]
    fn plans_images_for_projects_with_docker_config_or_image_name() {
        let (projects, tasks) = fixture();
        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );

        assert_eq!(
            images
                .iter()
                .map(|i| (i.project.as_str(), i.image_ref.as_str()))
                .collect::<Vec<_>>(),
            [("api", "acme/api"), ("web", "web")]
        );
    }

    #[test]
    fn builds_with_moons_dockerfile_and_the_whole_workspace_as_context() {
        let (projects, tasks) = fixture();
        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );
        let web = images.iter().find(|i| i.project == "web").unwrap();

        assert_eq!(web.dockerfile, ".moon/cache/tilt/web.Dockerfile");
        assert_eq!(
            web.command,
            "moon docker file web ../.moon/cache/tilt/web.Dockerfile --defaults && docker build -t $EXPECTED_REF -f .moon/cache/tilt/web.Dockerfile ."
        );
    }

    #[test]
    fn rebuilds_only_for_the_project_and_its_dependencies() {
        let (projects, tasks) = fixture();
        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );
        let web = images.iter().find(|i| i.project == "web").unwrap();
        let api = images.iter().find(|i| i.project == "api").unwrap();

        // web -> api-client -> api -> db, and web -> tooling (build scope).
        assert_eq!(
            web.watch,
            [
                ".moon/workspace.yml",
                "api",
                "api-client",
                "db",
                "tooling",
                "web"
            ]
        );
        assert_eq!(web.ignore, ["web/generated/**/*"]);

        // api only depends on db: web/docs changes never rebuild it.
        assert_eq!(api.watch, [".moon/workspace.yml", "api", "db"]);
    }

    #[test]
    fn builds_only_projects_that_opt_in() {
        let (mut projects, tasks) = fixture();
        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );
        let image = |images: &[Image], id: &str| images.iter().find(|i| i.project == id).cloned();

        assert!(image(&images, "web").is_some());
        assert!(image(&images, "api").is_some());

        // `docker.file` settings alone (e.g. for production images) opt nothing in.
        // Nor do `project.tilt.k8s` settings, which are no longer supported.
        let docs = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "docs")
            .unwrap();
        docs.config
            .docker
            .file
            .insert("image".into(), "node:22".into());
        docs.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(r#"{ "tilt": { "k8s": {} } }"#).unwrap(),
        });

        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );

        assert!(image(&images, "docs").is_none());
    }

    #[test]
    fn syncs_sources_and_rebuilds_for_manifests() {
        let (projects, tasks) = fixture();
        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );
        let web = images.iter().find(|i| i.project == "web").unwrap();
        let live_update = web.live_update.as_ref().unwrap();

        // web and everything it depends on, at the same path under /app.
        assert_eq!(
            live_update
                .syncs
                .iter()
                .map(|sync| (sync.local.as_str(), sync.remote.as_str()))
                .collect::<Vec<_>>(),
            [
                ("api", "/app/api"),
                ("api-client", "/app/api-client"),
                ("db", "/app/db"),
                ("tooling", "/app/tooling"),
                ("web", "/app/web"),
            ]
        );
        // moon config and explicit task inputs (web:dev's package.json) rebuild.
        assert!(
            live_update
                .fall_back_on
                .contains(&"web/package.json".to_owned())
        );
        assert!(
            live_update
                .fall_back_on
                .contains(&"web/moon.yml".to_owned())
        );
        assert!(
            !live_update
                .fall_back_on
                .iter()
                .any(|f| f.starts_with(".moon/extensions"))
        );
        // web reloads itself (default serviceRestart), so no restart.
        assert_eq!(live_update.restart_entrypoint, None);
    }

    #[test]
    fn restarts_apps_without_live_reload() {
        let (mut projects, tasks) = fixture();
        let web = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "web")
            .unwrap();
        web.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "image": "web", "serviceRestart": "inputs" } }"#,
            )
            .unwrap(),
        });

        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );
        let web = images.iter().find(|i| i.project == "web").unwrap();

        assert_eq!(
            web.live_update.as_ref().unwrap().restart_entrypoint,
            Some(vec!["moon".into(), "run".into(), "web:dev".into()])
        );
    }

    #[test]
    fn can_disable_live_update() {
        let (mut projects, tasks) = fixture();
        let web = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "web")
            .unwrap();
        web.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "image": "web", "liveUpdate": false } }"#,
            )
            .unwrap(),
        });

        let images = plan_images(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &config_files(),
        );

        assert!(
            images
                .iter()
                .find(|i| i.project == "web")
                .unwrap()
                .live_update
                .is_none()
        );
        assert!(
            images
                .iter()
                .find(|i| i.project == "api")
                .unwrap()
                .live_update
                .is_some()
        );

        let config = TiltExtensionConfig {
            live_update: false,
            ..Default::default()
        };

        assert!(
            plan_images(&projects, &tasks, &config, &config_files())
                .iter()
                .all(|image| image.live_update.is_none())
        );
    }

    #[test]
    fn can_disable_images() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            images: false,
            ..Default::default()
        };

        assert!(plan_images(&projects, &tasks, &config, &config_files()).is_empty());
    }

    #[test]
    fn parses_and_formats_tilt_args() {
        let args: Vec<String> = ["web", "--to-edit", "api", "docs", "--to-edit=web"]
            .map(String::from)
            .to_vec();
        let parsed = TiltArgs::parse(&args);

        assert_eq!(parsed.to_run, ["web", "docs"]);
        assert_eq!(parsed.to_edit, Some(vec!["api".into(), "web".into()]));
        assert_eq!(
            parsed.to_args(),
            ["web", "docs", "--to-edit", "api", "--to-edit", "web"]
        );
        assert_eq!(TiltArgs::parse(&["web".into()]).to_edit, None);
    }

    #[test]
    fn resolves_which_images_build_from_source() {
        let explicit = vec!["docs".to_owned()];

        assert_eq!(resolve_to_edit(&[], None), None);
        assert_eq!(resolve_to_edit(&explicit, None), Some(explicit.clone()));
        assert_eq!(
            resolve_to_edit(&explicit, Some(vec!["api".into()])),
            Some(vec!["api".into(), "docs".into()])
        );
        // Nothing affected: build nothing from source, use prebuilt images.
        assert_eq!(
            resolve_to_edit(&[], Some(vec![])),
            Some(vec!["none".into()])
        );
    }
}
