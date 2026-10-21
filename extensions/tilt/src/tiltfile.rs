//! Converts the moon task graph into Tilt `local_resource` definitions and
//! renders them as Starlark (through a Tera template). Pure logic, so it is
//! unit tested natively.

use crate::config::{ServiceRestart, TiltExtensionConfig};
use crate::query::{QueryProjectsOutput, QueryTask, QueryTasksOutput};
use crate::template::{self, TemplateContext, TemplateProject, TemplateProjectDependency};
use crate::watch::WatchRules;
use crate::workloads::{
    DeclaredWorkloads, DecoratedWorkload, declared_workloads, decorate_workloads,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// A moon task represented as a Tilt `local_resource`. Serialized as-is
/// into the template context (`resources`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Resource {
    /// moon target, used verbatim as the Tilt resource name.
    pub target: String,
    pub project: String,
    /// Task ID within the project.
    pub task: String,
    /// Command Tilt runs: `moon run <target> --upstream none`.
    pub cmd: Vec<String>,
    /// Tilt UI labels (the project ID).
    pub labels: Vec<String>,
    /// Long-running tasks use `serve_cmd`, everything else uses `cmd`.
    pub persistent: bool,
    /// Workspace-relative paths that Tilt should watch (`deps`).
    pub watch: Vec<String>,
    /// Ignore patterns applied before the workspace gitignore patterns:
    /// everything under the watched directories except the input globs.
    pub only_inputs: Vec<String>,
    /// Ignore patterns applied after the gitignore patterns: negated inputs,
    /// outputs, and `!`-re-included explicit input files.
    pub ignore: Vec<String>,
    /// Whether the workspace gitignore patterns apply (input-based watching).
    pub respect_gitignore: bool,
    /// Targets of upstream resources.
    pub resource_deps: Vec<String>,
    /// Extra environment variables, for dependencies moon runs with `env`.
    pub env: BTreeMap<String, String>,
}

/// Select tasks to expose to Tilt, then pull in their transitive
/// dependencies so Tilt (not moon) orchestrates the whole subgraph.
///
/// Task dependencies become `resource_deps` directly. Project dependencies
/// (moon's project graph) link each service to the nearest upstream services,
/// and optionally pull those services in even when they aren't tagged.
pub fn plan_resources(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
) -> Vec<Resource> {
    plan(projects, tasks, config, &BTreeMap::new()).resources
}

/// Everything the generated file is rendered from.
#[derive(Debug, Default)]
pub struct Planned {
    pub resources: Vec<Resource>,
    pub images: Vec<crate::images::Image>,
    pub workloads: Vec<DecoratedWorkload>,
    pub excluded: Vec<ExcludedTask>,
}

/// Plan local resources, container images, and workload decorations together.
/// Projects with workloads (declared in `project.tilt.workloads`) are
/// containerized: their services don't run on the host, and their workloads
/// join the dependency graph.
pub fn plan_with_images(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
    config_files: &[String],
) -> Planned {
    let mut images = crate::images::plan_images(projects, tasks, config, config_files);
    let containerized: BTreeMap<String, DeclaredWorkloads> = projects
        .projects
        .iter()
        .map(|project| (project.id.clone(), declared_workloads(project)))
        .filter(|(_, workloads)| !workloads.is_empty())
        .collect();

    // Tilt can't override the entrypoint of images that only Compose deploys,
    // so restart their containers instead of the process.
    for image in &mut images {
        let compose_only = containerized
            .get(&image.project)
            .is_some_and(|workloads| workloads.k8s.is_empty());

        if let Some(live_update) = image.live_update.as_mut().filter(|_| compose_only)
            && live_update.restart_entrypoint.take().is_some()
        {
            live_update.restart_container = true;
        }
    }

    let plan = plan(projects, tasks, config, &containerized);

    let workloads = decorate_workloads(projects, &plan.workload_deps, config.workload_labels);

    Planned {
        resources: plan.resources,
        images,
        workloads,
        excluded: plan.excluded,
    }
}

/// Resources, plus `resource_deps` for containerized projects' workloads.
#[derive(Debug, Default)]
pub struct Plan {
    pub resources: Vec<Resource>,
    /// Containerized project ID -> upstream resources that each of its
    /// workloads waits on, from the project graph.
    pub workload_deps: BTreeMap<String, Vec<String>>,
    /// Tasks left out because they depend on a task that runs Tilt, which
    /// would otherwise have been resources.
    pub excluded: Vec<ExcludedTask>,
}

/// A task left out because it depends on a task that runs Tilt. Serialized
/// as-is into the template context (`excluded`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExcludedTask {
    pub target: String,
    /// The dependency that runs Tilt, or that was itself excluded for it.
    pub dependency: String,
}

/// Like [`plan_resources`], where `containerized` projects run as the workloads
/// they declare: their workloads stand in as their services in the project
/// graph, and their tasks aren't local resources, except non-persistent tasks
/// that carry the tag themselves (build steps) and those tasks' dependencies.
///
/// Tasks that follow a task running Tilt are never resources. Those that would
/// otherwise have been (by planning without excluding them) are listed in
/// [`Plan::excluded`].
pub fn plan(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
    containerized: &BTreeMap<String, DeclaredWorkloads>,
) -> Plan {
    let follows_tilt = tasks_following_tilt(tasks);
    let mut plan = plan_excluding(projects, tasks, config, containerized, &follows_tilt);

    if !follows_tilt.is_empty() {
        let unexcluded = plan_excluding(projects, tasks, config, containerized, &BTreeMap::new());
        let mut excluded: Vec<ExcludedTask> = unexcluded
            .resources
            .iter()
            .filter_map(|resource| {
                let target = format!("{}:{}", resource.project, resource.task);
                let dependency = follows_tilt.get(target.as_str())?;

                Some(ExcludedTask {
                    dependency: dependency.to_string(),
                    target,
                })
            })
            .collect();

        excluded.sort_by(|a, b| a.target.cmp(&b.target));
        excluded.dedup();
        plan.excluded = excluded;
    }

    plan
}

/// Tasks that depend on a task that runs Tilt, directly or through other such
/// tasks, mapped to that dependency. They run after Tilt is up (e.g. opening
/// its UI), so as resources inside Tilt they would start out of order: Tilt
/// silently drops `resource_deps` on resources that don't exist.
fn tasks_following_tilt(tasks: &QueryTasksOutput) -> BTreeMap<&str, &str> {
    let all: BTreeMap<&str, &QueryTask> = tasks
        .tasks
        .values()
        .flat_map(|project_tasks| project_tasks.values())
        .map(|task| (task.target.as_str(), task))
        .collect();
    let mut follows: BTreeMap<&str, &str> = BTreeMap::new();

    // Repeat until no task is added, so chains of any length are covered.
    loop {
        let before = follows.len();

        for (target, task) in &all {
            if runs_tilt(task) || follows.contains_key(target) {
                continue;
            }

            let cause = task.deps.iter().find(|dep| {
                follows.contains_key(dep.target.as_str())
                    || all
                        .get(dep.target.as_str())
                        .is_some_and(|dep| runs_tilt(dep))
            });

            if let Some(dep) = cause {
                follows.insert(target, dep.target.as_str());
            }
        }

        if follows.len() == before {
            return follows;
        }
    }
}

/// [`plan`], leaving out the tasks in `excluded` entirely.
fn plan_excluding(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
    containerized: &BTreeMap<String, DeclaredWorkloads>,
    excluded: &BTreeMap<&str, &str>,
) -> Plan {
    let tagged_projects: BTreeSet<&str> = projects
        .projects
        .iter()
        .filter(|project| project.config.tags.iter().any(|tag| tag == &config.tag))
        .map(|project| project.id.as_str())
        .collect();

    let project_deps: BTreeMap<&str, Vec<&str>> = projects
        .projects
        .iter()
        .map(|project| {
            let deps = project
                .dependencies
                .iter()
                .flatten()
                .filter(|dep| config.dependency_scopes.contains(&dep.scope))
                .map(|dep| dep.id.as_str())
                .collect();

            (project.id.as_str(), deps)
        })
        .collect();

    let projects_by_id: BTreeMap<&str, &crate::query::QueryProject> = projects
        .projects
        .iter()
        .map(|project| (project.id.as_str(), project))
        .collect();

    let mut by_target: BTreeMap<&str, (&str, &QueryTask)> = BTreeMap::new();
    let mut services: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for (project_id, project_tasks) in &tasks.tasks {
        let is_containerized = containerized.contains_key(project_id);

        for task in project_tasks.values() {
            // The workload runs the project; its services don't run on the host.
            if is_containerized && task.options.persistent {
                continue;
            }

            // Not even a dependency: no resource may wait on it.
            if excluded.contains_key(task.target.as_str()) {
                continue;
            }

            by_target.insert(task.target.as_str(), (project_id.as_str(), task));

            if is_service(task) {
                services
                    .entry(project_id.as_str())
                    .or_default()
                    .push(task.target.as_str());
            }
        }
    }

    // A containerized project's services are its declared workloads.
    let workload_names: BTreeSet<&str> = containerized
        .values()
        .flat_map(|workloads| workloads.names())
        .collect();

    for (id, workloads) in containerized {
        if let Some((id, _)) = projects_by_id.get_key_value(id.as_str()) {
            services.insert(id, workloads.names().collect());
        }
    }

    let mut queue: Vec<&str> = by_target
        .iter()
        .filter(|(_, (project_id, task))| {
            if task.options.interactive || runs_tilt(task) {
                return false;
            }

            // Only build steps tagged themselves run for containerized projects.
            task.tags.iter().any(|tag| tag == &config.tag)
                || (is_service(task)
                    && (config.include_persistent || tagged_projects.contains(project_id)))
        })
        .map(|(target, _)| *target)
        .collect();

    // Workloads always deploy, so their upstream services must be present too.
    if config.link_project_dependencies && config.include_upstream_services {
        for id in projects_by_id
            .keys()
            .filter(|id| containerized.contains_key(**id))
        {
            queue.extend(upstream_services(id, &project_deps, |dep_id| {
                services.get(dep_id).cloned().unwrap_or_default()
            }));
        }
    }

    let mut selected: BTreeSet<&str> = BTreeSet::new();
    // Targets that run as-is (not only with dependency args/env).
    let mut plain: BTreeSet<&str> = queue.iter().copied().collect();

    while let Some(target) = queue.pop() {
        if !selected.insert(target) {
            continue;
        }

        let Some((project_id, task)) = by_target.get(target) else {
            continue;
        };

        for dep in &task.deps {
            if by_target
                .get(dep.target.as_str())
                .is_some_and(|(_, dep_task)| !runs_tilt(dep_task))
            {
                queue.push(dep.target.as_str());

                if !dep.is_parameterized() {
                    plain.insert(dep.target.as_str());
                }
            }
        }

        if is_service(task) && config.link_project_dependencies && config.include_upstream_services
        {
            let upstream = upstream_services(project_id, &project_deps, |id| {
                services.get(id).cloned().unwrap_or_default()
            });

            plain.extend(upstream.iter().copied());
            queue.extend(upstream);
        }
    }

    let mut resources: Vec<Resource> = selected
        .iter()
        .filter_map(|target| by_target.get(target))
        .map(|(project_id, task)| {
            let restart = projects_by_id
                .get(project_id)
                .and_then(|project| project.config.tilt_metadata())
                .and_then(|tilt| tilt.get("serviceRestart"))
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .unwrap_or(config.service_restart);
            let upstream: Vec<&QueryTask> = task
                .deps
                .iter()
                .filter_map(|dep| by_target.get(dep.target.as_str()))
                .map(|(_, dep_task)| *dep_task)
                .collect();
            let mut resource = create_resource(project_id, task, &upstream, restart, config);

            if is_service(task) && config.link_project_dependencies {
                // Only link to services that are actually exposed to Tilt.
                let upstream = upstream_services(project_id, &project_deps, |id| {
                    services
                        .get(id)
                        .into_iter()
                        .flatten()
                        .filter(|target| {
                            selected.contains(*target) || workload_names.contains(*target)
                        })
                        .copied()
                        .collect()
                });

                resource
                    .resource_deps
                    .extend(upstream.into_iter().map(String::from));
                resource.resource_deps.sort();
                resource.resource_deps.dedup();
            }

            // moon runs dependencies declared with args/env differently from
            // the plain target, so depend on a dedicated resource for them.
            for dep in task.deps.iter().filter(|dep| dep.is_parameterized()) {
                for existing in resource.resource_deps.iter_mut() {
                    if *existing == dep.target {
                        *existing = dep.resource_name();
                    }
                }
            }

            resource
        })
        .collect();

    // Create a resource per distinct parameterized dependency, cloned from the
    // plain one: `moon run <target> --upstream none -- <args>` with its env.
    let mut variants: BTreeMap<String, Resource> = BTreeMap::new();

    for target in &selected {
        let Some((_, task)) = by_target.get(target) else {
            continue;
        };

        for dep in task.deps.iter().filter(|dep| dep.is_parameterized()) {
            let Some(base) = resources.iter().find(|r| r.target == dep.target) else {
                continue;
            };

            variants.entry(dep.resource_name()).or_insert_with(|| {
                let mut variant = base.clone();
                variant.target = dep.resource_name();

                if !dep.args.is_empty() {
                    variant.cmd.push("--".into());
                    variant.cmd.extend(dep.args.iter().cloned());
                }

                variant.env.extend(dep.env.clone());
                variant
            });
        }
    }

    // Drop plain resources that are only ever run with args/env.
    resources.retain(|resource| plain.contains(resource.target.as_str()));
    resources.extend(variants.into_values());
    resources.sort_by(|a, b| a.target.cmp(&b.target));

    let workload_deps = projects_by_id
        .keys()
        .filter(|id| containerized.contains_key(**id))
        .map(|id| {
            let deps = if config.link_project_dependencies {
                upstream_services(id, &project_deps, |dep_id| {
                    services
                        .get(dep_id)
                        .into_iter()
                        .flatten()
                        .filter(|target| {
                            selected.contains(*target) || workload_names.contains(*target)
                        })
                        .copied()
                        .collect()
                })
                .into_iter()
                .map(String::from)
                .collect()
            } else {
                vec![]
            };

            (id.to_string(), deps)
        })
        .collect();

    Plan {
        resources,
        workload_deps,
        excluded: vec![],
    }
}

/// Long-running tasks that Tilt runs with `serve_cmd`.
fn is_service(task: &QueryTask) -> bool {
    task.options.persistent && !task.options.interactive && !runs_tilt(task)
}

/// Tasks that run Tilt itself (e.g. a `tilt up` task with the Tilt toolchain) are
/// never resources: that would nest Tilt in Tilt, and project Tiltfiles are
/// already `include()`d directly.
fn runs_tilt(task: &QueryTask) -> bool {
    task.command == "tilt" || task.toolchains.iter().any(|id| id == "tilt")
}

/// Walk the project graph upstream from `project_id` and return the services
/// of the nearest projects that have any. Projects without services (libraries)
/// are walked through, so `web -> api-client -> api` links `web` to `api`.
fn upstream_services<'a>(
    project_id: &'a str,
    project_deps: &BTreeMap<&'a str, Vec<&'a str>>,
    services_of: impl Fn(&str) -> Vec<&'a str>,
) -> Vec<&'a str> {
    let mut found = vec![];
    let mut visited = BTreeSet::from([project_id]);
    let mut stack: Vec<&str> = project_deps.get(project_id).cloned().unwrap_or_default();

    while let Some(dep_id) = stack.pop() {
        if !visited.insert(dep_id) {
            continue;
        }

        let dep_services = services_of(dep_id);

        if dep_services.is_empty() {
            stack.extend(project_deps.get(dep_id).into_iter().flatten());
        } else {
            found.extend(dep_services);
        }
    }

    found
}

fn create_resource(
    project_id: &str,
    task: &QueryTask,
    upstream: &[&QueryTask],
    restart: ServiceRestart,
    config: &TiltExtensionConfig,
) -> Resource {
    let watch_inputs = || {
        crate::watch::plan_watch(
            task.input_files.keys(),
            task.input_globs.keys(),
            task.output_files.keys(),
            task.output_globs.keys(),
        )
    };

    // Build steps re-run `moon run` when moon's inputs change (moon's cache
    // decides if any work is needed). Services are restarted per `restart`.
    let (rules, respect_gitignore) = match (config.watch_inputs, is_service(task), restart) {
        (false, _, _) | (_, true, ServiceRestart::Never) => (WatchRules::default(), false),
        (true, false, _) | (true, true, ServiceRestart::Inputs) => (watch_inputs(), true),
        // Outputs only change when moon actually rebuilds (cache hits leave
        // them untouched), and they're usually gitignored, so don't apply it.
        (true, true, ServiceRestart::UpstreamOutputs) => (
            crate::watch::plan_watch(
                upstream.iter().flat_map(|dep| dep.output_files.keys()),
                upstream.iter().flat_map(|dep| dep.output_globs.keys()),
                [],
                [],
            ),
            false,
        ),
    };

    Resource {
        target: task.target.clone(),
        project: project_id.to_owned(),
        task: task
            .target
            .rsplit_once(':')
            .map(|(_, id)| id.to_owned())
            .unwrap_or_default(),
        // `moon run` is `moon exec --on-failure bail`, but skips tasks with
        // `runInCI: false` (the default for persistent tasks) when `CI` is set.
        // Tilt already decided what runs, so ignore moon's CI checks.
        cmd: vec![
            config.moon_bin.clone(),
            "exec".into(),
            task.target.clone(),
            "--upstream".into(),
            "none".into(),
            "--on-failure".into(),
            "bail".into(),
            "--ignore-ci-checks".into(),
        ],
        labels: vec![project_id.to_owned()],
        persistent: task.options.persistent,
        watch: rules.watch,
        only_inputs: rules.only_inputs,
        ignore: rules.ignore,
        respect_gitignore,
        resource_deps: task.deps.iter().map(|dep| dep.target.clone()).collect(),
        env: BTreeMap::new(),
    }
}

/// Workspace-relative paths of project Tiltfiles to include, excluding the
/// main and generated files (which would include themselves).
pub fn find_project_tiltfiles(
    projects: &QueryProjectsOutput,
    config: &TiltExtensionConfig,
    exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    if !config.include_project_tiltfiles {
        return vec![];
    }

    let mut paths: Vec<String> = projects
        .projects
        .iter()
        .map(|project| match project.source.trim_matches('/') {
            "" | "." => "Tiltfile".to_owned(),
            source => format!("{source}/Tiltfile"),
        })
        .filter(|path| {
            normalize(path) != normalize(&config.tiltfile)
                && normalize(path) != normalize(&config.output)
                && exists(path)
        })
        .collect();

    paths.sort();
    paths.dedup();
    paths
}

fn normalize(path: &str) -> &str {
    path.trim_start_matches("./")
}

/// Relative path from the directory containing `file` back to the workspace root.
pub fn path_to_root(file: &str) -> String {
    let depth = file
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .count()
        .saturating_sub(1);

    if depth == 0 {
        ".".into()
    } else {
        vec![".."; depth].join("/")
    }
}

/// Starlark string literal. JSON string escaping is a compatible subset.
fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("strings always serialize")
}

/// Render the generated Tiltfile with the built-in Tera template, or a custom one.
/// Everything the generated file is rendered from, besides the config.
pub struct RenderInput<'a> {
    pub resources: &'a [Resource],
    pub projects: &'a QueryProjectsOutput,
    /// Workspace-relative project Tiltfiles to `include()`.
    pub project_tiltfiles: &'a [String],
    /// Workspace-relative ignore patterns translated from `.gitignore` files.
    pub gitignore: &'a [String],
    /// Workspace-relative files the generated file is derived from (moon
    /// config, `.gitignore` files, the template). Watched to regenerate.
    pub config_files: &'a [String],
    /// ID the extension is configured with in `.moon/extensions.yml`.
    pub extension_id: &'a str,
    /// Container images to build.
    pub images: &'a [crate::images::Image],
    /// Workloads deployed by project Tiltfiles, to decorate.
    pub workloads: &'a [DecoratedWorkload],
    /// Tasks left out because they follow a task that runs Tilt.
    pub excluded: &'a [ExcludedTask],
    pub version: &'a str,
    pub custom_template: Option<&'a str>,
}

impl<'a> RenderInput<'a> {
    pub fn new(
        resources: &'a [Resource],
        projects: &'a QueryProjectsOutput,
        version: &'a str,
    ) -> Self {
        Self {
            resources,
            projects,
            project_tiltfiles: &[],
            gitignore: &[],
            config_files: &[],
            extension_id: "tilt",
            images: &[],
            workloads: &[],
            excluded: &[],
            version,
            custom_template: None,
        }
    }
}

/// Paths that must stay watched even though they're gitignored, re-included
/// in the global `watch_settings()` ignore (a resource's own `!` patterns can't
/// override a global ignore): explicit input files (moon hashes them even
/// when gitignored), paths services watch for upstream outputs (usually
/// gitignored build artifacts), and `always_watched` files: the generation
/// inputs and Tilt's own config (the global ignore also hides Tiltfile
/// changes from Tilt, and generated Tiltfiles are often gitignored).
pub fn gitignore_exceptions(resources: &[Resource], always_watched: &[String]) -> Vec<String> {
    let mut exceptions: BTreeSet<String> = always_watched
        .iter()
        .map(|file| format!("!{file}"))
        .collect();

    for resource in resources {
        exceptions.extend(
            resource
                .ignore
                .iter()
                .filter(|pattern| pattern.starts_with('!'))
                .cloned(),
        );

        if !resource.respect_gitignore {
            for path in &resource.watch {
                exceptions.insert(format!("!{path}"));
                exceptions.insert(format!("!{path}/**"));
            }
        }
    }

    exceptions.into_iter().collect()
}

/// Render the generated Tiltfile with the built-in Tera template, or a custom one.
pub fn render(input: &RenderInput, config: &TiltExtensionConfig) -> Result<String, String> {
    let RenderInput {
        resources,
        project_tiltfiles,
        gitignore,
        config_files,
        ..
    } = *input;
    let projects = input
        .projects
        .projects
        .iter()
        .map(|project| {
            let tiltfile = match project.source.trim_matches('/') {
                "" | "." => "Tiltfile".to_owned(),
                source => format!("{source}/Tiltfile"),
            };

            let own_resources: Vec<String> = resources
                .iter()
                .filter(|resource| resource.project == project.id)
                .map(|resource| resource.target.clone())
                .chain(declared_workloads(project).names().map(String::from))
                .collect();

            TemplateProject {
                id: project.id.clone(),
                source: project.source.clone(),
                tags: project.config.tags.clone(),
                dependencies: project
                    .dependencies
                    .iter()
                    .flatten()
                    .map(|dep| TemplateProjectDependency {
                        id: dep.id.clone(),
                        scope: dep.scope.clone(),
                    })
                    .collect(),
                tiltfile: project_tiltfiles.contains(&tiltfile).then_some(tiltfile),
                resources: own_resources.clone(),
                selection: own_resources
                    .into_iter()
                    .chain(
                        project
                            .config
                            .tilt_metadata()
                            .and_then(|tilt| tilt.get("resources"))
                            .and_then(|value| value.as_array())
                            .into_iter()
                            .flatten()
                            .filter_map(|value| value.as_str().map(String::from)),
                    )
                    .fold(vec![], |mut selection, name| {
                        if !selection.contains(&name) {
                            selection.push(name);
                        }
                        selection
                    }),
            }
        })
        .collect();

    let mut always_watched = config_files.to_vec();
    always_watched.push(config.output.clone());
    always_watched.push(config.tiltfile.clone());
    always_watched.extend(project_tiltfiles.iter().cloned());

    let gitignore_exceptions = gitignore_exceptions(resources, &always_watched);
    let regenerate_cmd = vec![
        config.moon_bin.clone(),
        "ext".into(),
        input.extension_id.to_owned(),
        "--".into(),
        "generate".into(),
    ];

    template::render(
        &TemplateContext {
            version: input.version,
            root: path_to_root(&config.output),
            output: &config.output,
            tag: &config.tag,
            project_tiltfiles,
            gitignore,
            gitignore_exceptions: &gitignore_exceptions,
            global_gitignore: config.global_gitignore,
            config_files,
            regenerate_cmd,
            watch_config: config.watch_config,
            select_by_project: config.select_by_project,
            resources,
            projects,
            images: input.images,
            workloads: input.workloads,
            excluded: input.excluded,
            restart_process: input.images.iter().any(|image| {
                image
                    .live_update
                    .as_ref()
                    .is_some_and(|live_update| live_update.restart_entrypoint.is_some())
            }),
        },
        input.custom_template,
    )
}

/// Contents for a main Tiltfile that includes the generated file.
pub fn render_main_tiltfile(config: &TiltExtensionConfig) -> String {
    let root = path_to_root(&config.tiltfile);
    let include = if root == "." {
        format!("./{}", config.output)
    } else {
        format!("{root}/{}", config.output)
    };

    format!(
        "# Project Tiltfiles and moon task resources, kept in sync by moon.\n\
         # `tilt up` from here runs the entire cluster.\n\
         include({})\n\n\
         # Add your own resources (k8s_yaml, docker_build, ...) below.\n",
        quote(&include)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (QueryProjectsOutput, QueryTasksOutput) {
        let projects =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/projects.json"))
                .unwrap();
        let tasks =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/tasks.json")).unwrap();

        (projects, tasks)
    }

    fn targets(resources: &[Resource]) -> Vec<&str> {
        resources.iter().map(|r| r.target.as_str()).collect()
    }

    #[test]
    fn computes_path_to_root() {
        assert_eq!(path_to_root("Tiltfile.moon"), ".");
        assert_eq!(path_to_root("./Tiltfile.moon"), ".");
        assert_eq!(path_to_root(".moon/tilt/Tiltfile"), "../..");
    }

    #[test]
    fn selects_tagged_projects_persistent_tasks_and_their_deps() {
        let (projects, tasks) = fixture();
        let resources = plan_resources(&projects, &tasks, &TiltExtensionConfig::default());

        // web is tagged: its server + codegen dep; api:serve is tagged directly
        // and pulls in api:build; db:serve is an upstream service of api.
        // Non-persistent web:lint, interactive web:repl, and the build-scoped
        // tooling:watch are excluded.
        assert_eq!(
            targets(&resources),
            [
                "api:build",
                "api:serve",
                "db:serve",
                "web:codegen",
                "web:dev"
            ]
        );
    }

    #[test]
    fn can_include_all_persistent_tasks() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            tag: "nothing-has-this".into(),
            include_persistent: true,
            ..Default::default()
        };

        assert_eq!(
            targets(&plan_resources(&projects, &tasks, &config)),
            [
                "api:build",
                "api:serve",
                "db:serve",
                "docs:serve",
                "tooling:watch",
                "web:codegen",
                "web:dev"
            ]
        );
    }

    #[test]
    fn maps_inputs_and_outputs_to_watch_and_ignore() {
        let (projects, tasks) = fixture();
        let resources = plan_resources(&projects, &tasks, &TiltExtensionConfig::default());
        let dev = resources.iter().find(|r| r.target == "web:dev").unwrap();
        let codegen = resources
            .iter()
            .find(|r| r.target == "web:codegen")
            .unwrap();

        // Build steps watch exactly moon's inputs, minus their outputs.
        assert!(!codegen.persistent);
        assert_eq!(codegen.watch, ["web"]);
        assert_eq!(codegen.only_inputs, ["web/**", "!web/**/*"]);
        assert_eq!(codegen.ignore, ["web/generated/**/*"]);
        assert!(codegen.respect_gitignore);

        // Services restart only when an upstream build's outputs change,
        // leaving source edits to their own live reload.
        assert!(dev.persistent);
        assert_eq!(dev.watch, ["web/generated"]);
        assert!(!dev.respect_gitignore);
        assert_eq!(dev.resource_deps, ["api:serve", "web:codegen"]);
    }

    #[test]
    fn services_without_upstream_outputs_never_restart() {
        let (projects, tasks) = fixture();
        let resources = plan_resources(&projects, &tasks, &TiltExtensionConfig::default());

        // api:build and db have no outputs.
        assert!(resource(&resources, "api:serve").watch.is_empty());
        assert!(resource(&resources, "db:serve").watch.is_empty());
    }

    #[test]
    fn can_restart_services_on_their_inputs() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            service_restart: ServiceRestart::Inputs,
            ..Default::default()
        };
        let resources = plan_resources(&projects, &tasks, &config);
        let dev = resource(&resources, "web:dev");

        assert_eq!(dev.watch, ["web/package.json", "web/src"]);
        assert!(dev.respect_gitignore);
    }

    #[test]
    fn can_never_restart_services() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            service_restart: ServiceRestart::Never,
            ..Default::default()
        };
        let resources = plan_resources(&projects, &tasks, &config);

        assert!(resource(&resources, "web:dev").watch.is_empty());
        // Build steps still watch their inputs.
        assert_eq!(resource(&resources, "web:codegen").watch, ["web"]);
    }

    #[test]
    fn projects_can_override_service_restart_in_metadata() {
        let (mut projects, tasks) = fixture();
        let web = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "web")
            .unwrap();

        web.config.project = Some(crate::query::QueryProjectMetadata {
            metadata: serde_json::from_str(r#"{ "tilt": { "serviceRestart": "inputs" } }"#)
                .unwrap(),
        });

        let resources = plan_resources(&projects, &tasks, &TiltExtensionConfig::default());

        assert_eq!(
            resource(&resources, "web:dev").watch,
            ["web/package.json", "web/src"]
        );
        // Other projects keep the default.
        assert!(resource(&resources, "api:serve").watch.is_empty());
    }

    fn resource<'a>(resources: &'a [Resource], target: &str) -> &'a Resource {
        resources.iter().find(|r| r.target == target).unwrap()
    }

    #[test]
    fn links_services_across_project_dependencies() {
        let (projects, tasks) = fixture();
        let resources = plan_resources(&projects, &tasks, &TiltExtensionConfig::default());

        // web -> api-client (library, no services) -> api
        assert_eq!(
            resource(&resources, "web:dev").resource_deps,
            ["api:serve", "web:codegen"]
        );
        // api -> db, plus the task dependency on api:build
        assert_eq!(
            resource(&resources, "api:serve").resource_deps,
            ["api:build", "db:serve"]
        );
        assert!(resource(&resources, "db:serve").resource_deps.is_empty());
        // Non-service resources only follow task dependencies.
        assert!(resource(&resources, "web:codegen").resource_deps.is_empty());
    }

    #[test]
    fn ignores_build_scoped_project_dependencies() {
        let (projects, tasks) = fixture();
        let resources = plan_resources(&projects, &tasks, &TiltExtensionConfig::default());

        assert!(!targets(&resources).contains(&"tooling:watch"));
    }

    #[test]
    fn can_customize_dependency_scopes() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            dependency_scopes: vec!["production".into(), "build".into()],
            ..Default::default()
        };
        let resources = plan_resources(&projects, &tasks, &config);

        assert_eq!(
            resource(&resources, "web:dev").resource_deps,
            ["api:serve", "tooling:watch", "web:codegen"]
        );
    }

    #[test]
    fn can_link_without_including_untagged_services() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            include_upstream_services: false,
            ..Default::default()
        };
        let resources = plan_resources(&projects, &tasks, &config);

        assert_eq!(
            targets(&resources),
            ["api:build", "api:serve", "web:codegen", "web:dev"]
        );
        // api:serve is selected (tagged), so web:dev still waits on it.
        assert_eq!(
            resource(&resources, "web:dev").resource_deps,
            ["api:serve", "web:codegen"]
        );
        // db:serve is not exposed, so there's nothing to wait on.
        assert_eq!(
            resource(&resources, "api:serve").resource_deps,
            ["api:build"]
        );
    }

    #[test]
    fn can_disable_project_dependency_links() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            link_project_dependencies: false,
            ..Default::default()
        };
        let resources = plan_resources(&projects, &tasks, &config);

        assert_eq!(
            targets(&resources),
            ["api:build", "api:serve", "web:codegen", "web:dev"]
        );
        assert_eq!(
            resource(&resources, "web:dev").resource_deps,
            ["web:codegen"]
        );
    }

    #[test]
    fn runs_parameterized_dependencies_as_their_own_resource() {
        let (projects, mut tasks) = fixture();
        let dev = tasks.tasks.get_mut("web").unwrap().get_mut("dev").unwrap();
        let dep = dev
            .deps
            .iter_mut()
            .find(|d| d.target == "web:codegen")
            .unwrap();

        dep.args = vec!["--prod".into()];
        dep.env = BTreeMap::from([("MODE".into(), "x".into())]);

        let config = TiltExtensionConfig::default();
        let resources = plan_resources(&projects, &tasks, &config);
        let variant = resource(&resources, "web:codegen[--prod MODE=x]");

        assert_eq!(
            variant.cmd,
            [
                "moon",
                "exec",
                "web:codegen",
                "--upstream",
                "none",
                "--on-failure",
                "bail",
                "--ignore-ci-checks",
                "--",
                "--prod"
            ]
        );
        assert_eq!(variant.env, BTreeMap::from([("MODE".into(), "x".into())]));
        assert_eq!(
            resource(&resources, "web:dev").resource_deps,
            ["api:serve", "web:codegen[--prod MODE=x]"]
        );
        // web:codegen is only ever run with args, so no plain resource.
        assert!(!targets(&resources).contains(&"web:codegen"));

        let output = render(&RenderInput::new(&resources, &projects, "0.0.0"), &config).unwrap();

        assert!(output.contains(
            "    env = dict(list(_MOON_ENV.items()) + list({\"MODE\": \"x\"}.items())),\n"
        ));
    }

    #[test]
    fn never_exposes_tasks_that_run_tilt() {
        let (projects, mut tasks) = fixture();
        let db = tasks.tasks.get_mut("db").unwrap();

        // Like the toolchain's inferred task for a project with a Tiltfile.
        db.insert(
            "tilt-up".into(),
            QueryTask {
                target: "db:tilt-up".into(),
                command: "tilt".into(),
                toolchains: vec!["tilt".into()],
                options: crate::query::QueryTaskOptions {
                    persistent: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        let config = TiltExtensionConfig {
            include_persistent: true,
            ..Default::default()
        };
        let resources = plan_resources(&projects, &tasks, &config);

        assert!(!targets(&resources).contains(&"db:tilt-up"));
        assert_eq!(
            resource(&resources, "api:serve").resource_deps,
            ["api:build", "db:serve"]
        );
    }

    /// Add a task to the fixture: `command` `tilt` runs Tilt.
    fn add_task(
        tasks: &mut QueryTasksOutput,
        target: &str,
        command: &str,
        persistent: bool,
        deps: &[&str],
        tags: &[&str],
    ) {
        let (project, id) = target.split_once(':').unwrap();

        tasks.tasks.entry(project.into()).or_default().insert(
            id.into(),
            QueryTask {
                target: target.into(),
                command: command.into(),
                deps: deps
                    .iter()
                    .map(|dep| crate::query::QueryTaskDep {
                        target: dep.to_string(),
                        ..Default::default()
                    })
                    .collect(),
                options: crate::query::QueryTaskOptions {
                    persistent,
                    ..Default::default()
                },
                tags: tags.iter().map(|tag| tag.to_string()).collect(),
                ..Default::default()
            },
        );
    }

    fn excluded(plan: &Plan) -> Vec<(&str, &str)> {
        plan.excluded
            .iter()
            .map(|task| (task.target.as_str(), task.dependency.as_str()))
            .collect()
    }

    #[test]
    fn excludes_tasks_that_follow_a_tilt_task() {
        let (projects, mut tasks) = fixture();
        add_task(&mut tasks, "web:tilt-up", "tilt", true, &[], &[]);
        add_task(
            &mut tasks,
            "web:tilt-open",
            "open",
            true,
            &["web:tilt-up"],
            &[],
        );

        let plan = plan(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &BTreeMap::new(),
        );
        let resources = targets(&plan.resources);

        // web is tagged, so tilt-open (persistent) would have been a resource.
        assert!(!resources.contains(&"web:tilt-up"));
        assert!(!resources.contains(&"web:tilt-open"));
        assert_eq!(excluded(&plan), [("web:tilt-open", "web:tilt-up")]);
        // The rest of the project is unaffected, and nothing waits on them.
        assert!(resources.contains(&"web:dev"));
        assert!(plan.resources.iter().all(|resource| {
            !resource
                .resource_deps
                .iter()
                .any(|dep| dep.starts_with("web:tilt-"))
        }));
    }

    #[test]
    fn excludes_tagged_tasks_that_follow_a_tilt_task() {
        let (projects, mut tasks) = fixture();
        add_task(&mut tasks, "api:tilt-up", "tilt", true, &[], &[]);
        add_task(
            &mut tasks,
            "api:open-ui",
            "open",
            false,
            &["api:tilt-up"],
            &["tilt"],
        );

        let plan = plan(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &BTreeMap::new(),
        );

        assert!(!targets(&plan.resources).contains(&"api:open-ui"));
        assert_eq!(excluded(&plan), [("api:open-ui", "api:tilt-up")]);
        assert!(targets(&plan.resources).contains(&"api:serve"));
    }

    #[test]
    fn excludes_chains_of_tasks_after_a_tilt_task() {
        let (projects, mut tasks) = fixture();
        add_task(&mut tasks, "web:tilt-up", "tilt", true, &[], &[]);
        add_task(
            &mut tasks,
            "web:tilt-open",
            "open",
            true,
            &["web:tilt-up"],
            &[],
        );
        add_task(
            &mut tasks,
            "web:notify",
            "notify",
            true,
            &["web:tilt-open"],
            &[],
        );

        let plan = plan(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &BTreeMap::new(),
        );

        assert!(!targets(&plan.resources).contains(&"web:notify"));
        assert_eq!(
            excluded(&plan),
            [
                ("web:notify", "web:tilt-open"),
                ("web:tilt-open", "web:tilt-up")
            ]
        );
    }

    #[test]
    fn lists_only_exclusions_that_would_have_been_resources() {
        let (projects, mut tasks) = fixture();
        // docs isn't tagged, so nothing selects these anyway.
        add_task(&mut tasks, "docs:tilt-up", "tilt", true, &[], &[]);
        add_task(
            &mut tasks,
            "docs:tilt-open",
            "open",
            true,
            &["docs:tilt-up"],
            &[],
        );

        let plan = plan(
            &projects,
            &tasks,
            &TiltExtensionConfig::default(),
            &BTreeMap::new(),
        );

        assert!(plan.excluded.is_empty());

        // With every persistent task selected, it would have been one.
        let config = TiltExtensionConfig {
            include_persistent: true,
            ..Default::default()
        };
        let plan = super::plan(&projects, &tasks, &config, &BTreeMap::new());

        assert_eq!(excluded(&plan), [("docs:tilt-open", "docs:tilt-up")]);
        assert!(!targets(&plan.resources).contains(&"docs:tilt-open"));
    }

    #[test]
    fn renders_excluded_tasks_as_a_comment() {
        let (projects, mut tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let render_planned = |tasks: &QueryTasksOutput| {
            let planned = plan_with_images(&projects, tasks, &config, &[]);

            render(
                &RenderInput {
                    excluded: &planned.excluded,
                    ..RenderInput::new(&planned.resources, &projects, "0.0.0")
                },
                &config,
            )
            .unwrap()
        };

        assert!(!render_planned(&tasks).contains("Not exposed"));

        add_task(&mut tasks, "web:tilt-up", "tilt", true, &[], &[]);
        add_task(
            &mut tasks,
            "web:tilt-open",
            "open",
            true,
            &["web:tilt-up"],
            &[],
        );
        add_task(
            &mut tasks,
            "web:notify",
            "notify",
            true,
            &["web:tilt-open"],
            &[],
        );

        let output = render_planned(&tasks);

        assert!(output.contains(
            "\n# Not exposed: these tasks depend on a task that runs Tilt (directly, or\n# through another task listed here), so in Tilt they would start before it.\n#   web:notify (depends on web:tilt-open)\n#   web:tilt-open (depends on web:tilt-up)\n"
        ));
        assert!(!output.contains("local_resource(\n    \"web:tilt-open\""));
    }

    #[test]
    fn renders_project_selection_for_tilt_args() {
        let (mut projects, tasks) = fixture();
        let docs = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "docs")
            .unwrap();

        // Resources defined in the project's own Tiltfile.
        docs.config.project = Some(crate::query::QueryProjectMetadata {
            metadata: serde_json::from_str(r#"{ "tilt": { "resources": ["docs-site"] } }"#)
                .unwrap(),
        });

        let config = TiltExtensionConfig::default();
        let resources = plan_resources(&projects, &tasks, &config);
        let output = render(&RenderInput::new(&resources, &projects, "0.0.0"), &config).unwrap();

        assert!(output.contains(
            "_MOON_PROJECTS = {\n    \"api\": [\"api:build\", \"api:serve\"],\n    \"db\": [\"db:serve\"],\n    \"docs\": [\"docs-site\"],\n    \"web\": [\"web:codegen\", \"web:dev\"],\n}\n"
        ));
        assert!(output.contains("config.define_string_list(\"to-run\", args = True"));
        assert!(output.contains("config.set_enabled_resources(_moon_enabled)"));

        let config = TiltExtensionConfig {
            select_by_project: false,
            ..Default::default()
        };
        let output = render(&RenderInput::new(&resources, &projects, "0.0.0"), &config).unwrap();

        assert!(!output.contains("config.parse()"));
    }

    #[test]
    fn selects_declared_workloads_by_project() {
        let (mut projects, tasks) = fixture();
        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();

        // Listing a workload in `project.tilt.resources` too doesn't repeat it.
        api.config.project = Some(crate::query::QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "workloads": { "k8s": ["api", "api-worker"] }, "resources": ["api"] } }"#,
            )
            .unwrap(),
        });

        let config = TiltExtensionConfig::default();
        let Planned {
            resources, images, ..
        } = plan_with_images(&projects, &tasks, &config, &[]);
        let output = render(
            &RenderInput {
                images: &images,
                ..RenderInput::new(&resources, &projects, "0.0.0")
            },
            &config,
        )
        .unwrap();

        assert!(output.contains("    \"api\": [\"api\", \"api-worker\"],\n"));
    }

    #[test]
    fn computes_gitignore_exceptions() {
        let (projects, mut tasks) = fixture();

        // An explicit input file, which moon hashes even when gitignored.
        tasks
            .tasks
            .get_mut("api")
            .unwrap()
            .get_mut("build")
            .unwrap()
            .input_files
            .insert(".env".into(), serde_json::Value::Null);

        let resources = plan_resources(&projects, &tasks, &TiltExtensionConfig::default());
        let exceptions = gitignore_exceptions(&resources, &[".moon/workspace.yml".into()]);

        assert_eq!(
            exceptions,
            [
                "!.env",                // explicit input of api:build
                "!.moon/workspace.yml", // generation input
                "!web/generated",       // web:dev watches codegen's outputs
                "!web/generated/**",
            ]
        );
    }

    #[test]
    fn renders_global_gitignore_and_regenerate_resource() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let resources = plan_resources(&projects, &tasks, &config);
        let gitignore = ["**/node_modules".to_owned()];
        let config_files = [".moon/workspace.yml".to_owned(), "web/moon.yml".to_owned()];
        let input = RenderInput {
            gitignore: &gitignore,
            config_files: &config_files,
            extension_id: "tilt-dev",
            ..RenderInput::new(&resources, &projects, "0.0.0")
        };
        let output = render(&input, &config).unwrap();

        assert!(output.contains(
            "watch_settings(ignore = _MOON_GITIGNORE + [\"!\" + _moon_path(\".moon/workspace.yml\"), \"!\" + _moon_path(\"Tiltfile\"), \"!\" + _moon_path(\"Tiltfile.moon\"), \"!\" + _moon_path(\"web/generated\"), \"!\" + _moon_path(\"web/generated/**\"), \"!\" + _moon_path(\"web/moon.yml\")])\n"
        ));
        assert!(output.contains(
            "local_resource(\n    \"(moon)\",\n    cmd = [\"moon\", \"ext\", \"tilt-dev\", \"--\", \"generate\"],\n    dir = _MOON_ROOT,\n    env = _MOON_ENV,\n    deps = [_moon_path(\".moon/workspace.yml\"), _moon_path(\"web/moon.yml\")],\n    auto_init = False,\n)\n"
        ));
        // Selecting projects keeps the regenerate resource enabled.
        assert!(output.contains("if _moon_enabled:\n    _moon_enabled.append(\"(moon)\")\n"));

        let config = TiltExtensionConfig {
            global_gitignore: false,
            watch_config: false,
            ..Default::default()
        };
        let output = render(&input, &config).unwrap();

        assert!(!output.contains("watch_settings("));
        assert!(!output.contains("\"(moon)\""));
    }

    #[test]
    fn readme_from_scratch_template_defines_what_the_macro_uses() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let resources = plan_resources(&projects, &tasks, &config);
        let gitignore = ["**/node_modules".to_owned()];
        // The README example, over all resources (build steps reference
        // `_MOON_GITIGNORE` through the macro).
        let template = r#"{% import "moon/macros.tera" as moon -%}
_MOON_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), {{ root | starlark }}))
def _moon_path(path):
    return os.path.join(_MOON_ROOT, path)
_MOON_ENV = {"MOON_SKIP_SYNC_WORKSPACE": "true"}
_MOON_GITIGNORE = {{ gitignore | moon_path }}

{% for resource in resources -%}
{{ moon::resource(resource=resource) }}
{% endfor %}"#;
        let output = render(
            &RenderInput {
                gitignore: &gitignore,
                custom_template: Some(template),
                ..RenderInput::new(&resources, &projects, "0.0.0")
            },
            &config,
        )
        .unwrap();

        let defined = output.find("_MOON_GITIGNORE = [").unwrap();
        let used = output.find("+ _MOON_GITIGNORE").unwrap();

        assert!(defined < used);
    }

    #[test]
    fn renders_image_builds_gated_by_to_edit() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let resources = plan_resources(&projects, &tasks, &config);
        let images = [crate::images::Image {
            project: "web".into(),
            // Registry prefixes are kept verbatim as the `custom_build` ref.
            image_ref: "tilt.dev/web".into(),
            dockerfile: ".moon/cache/tilt/web.Dockerfile".into(),
            command: "moon docker file web ../.moon/cache/tilt/web.Dockerfile --defaults && docker build -t $EXPECTED_REF -f .moon/cache/tilt/web.Dockerfile .".into(),
            watch: vec!["web".into()],
            ignore: vec!["web/generated/**/*".into()],
            live_update: None,
        }];
        let output = render(
            &RenderInput {
                images: &images,
                ..RenderInput::new(&resources, &projects, "0.0.0")
            },
            &config,
        )
        .unwrap();

        assert!(output.contains("config.define_string_list(\"to-edit\""));
        assert!(output.contains("_moon_cfg = config.parse()\n"));
        assert!(output.contains(
            "if _moon_edit(\"web\"):\n    custom_build(\n        \"tilt.dev/web\",\n        \"moon docker file web ../.moon/cache/tilt/web.Dockerfile --defaults && docker build -t $EXPECTED_REF -f .moon/cache/tilt/web.Dockerfile .\",\n        deps = [_moon_path(\"web\")],\n        ignore = [_moon_path(\"web/generated/**/*\")],\n        dir = _MOON_ROOT,\n        env = _MOON_ENV,\n    )\n"
        ));
        assert!(output.contains("for _moon_name in _moon_cfg.get(\"to-run\", []):"));
    }

    #[test]
    fn compose_deployed_images_restart_their_container() {
        let (mut projects, tasks) = fixture();
        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();

        api.config
            .docker
            .file
            .insert("startTask".into(), "serve".into());
        api.config.project = Some(crate::query::QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "image": "api", "serviceRestart": "inputs", "workloads": { "compose": ["api"] } } }"#,
            )
            .unwrap(),
        });

        let config = TiltExtensionConfig::default();
        let Planned {
            resources,
            images,
            workloads,
            ..
        } = plan_with_images(&projects, &tasks, &config, &[]);
        let live_update = images[0].live_update.as_ref().unwrap();

        assert_eq!(live_update.restart_entrypoint, None);
        assert!(live_update.restart_container);

        let output = render(
            &RenderInput {
                images: &images,
                workloads: &workloads,
                ..RenderInput::new(&resources, &projects, "0.0.0")
            },
            &config,
        )
        .unwrap();

        assert!(output.contains("    custom_build(\n        \"api\",\n"));
        assert!(!output.contains("restart_process"));
        assert!(!output.contains("entrypoint"));
        assert!(output.contains("            restart_container(),\n        ],\n"));

        // Kubernetes workloads keep restarting the process.
        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();
        api.config.project = Some(crate::query::QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "image": "api", "serviceRestart": "inputs", "workloads": { "k8s": ["api"] } } }"#,
            )
            .unwrap(),
        });

        let images = plan_with_images(&projects, &tasks, &config, &[]).images;
        let live_update = images[0].live_update.as_ref().unwrap();

        assert!(live_update.restart_entrypoint.is_some());
        assert!(!live_update.restart_container);
    }

    #[test]
    fn images_build_without_deploying() {
        let (mut projects, tasks) = fixture();
        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();

        api.config.project = Some(crate::query::QueryProjectMetadata {
            metadata: serde_json::from_str(r#"{ "tilt": { "image": "tilt.dev/api" } }"#).unwrap(),
        });

        let config = TiltExtensionConfig::default();
        let Planned {
            resources,
            images,
            workloads,
            ..
        } = plan_with_images(&projects, &tasks, &config, &[]);
        let output = render(
            &RenderInput {
                images: &images,
                workloads: &workloads,
                ..RenderInput::new(&resources, &projects, "0.0.0")
            },
            &config,
        )
        .unwrap();

        assert!(output.contains("    custom_build(\n        \"tilt.dev/api\",\n"));
        assert!(!output.contains("k8s_yaml"));
        assert!(!output.contains("k8s_resource"));
        // An image alone doesn't containerize: api's services still run locally.
        assert!(targets(&resources).contains(&"api:serve"));
    }

    fn k8s(project: &str, names: &[&str]) -> BTreeMap<String, DeclaredWorkloads> {
        BTreeMap::from([(
            project.to_owned(),
            DeclaredWorkloads {
                k8s: names.iter().map(|name| name.to_string()).collect(),
                ..Default::default()
            },
        )])
    }

    #[test]
    fn containerized_projects_run_as_workloads_in_the_graph() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let plan = plan(
            &projects,
            &tasks,
            &config,
            &k8s("api", &["api", "api-worker"]),
        );

        // api's tasks don't run on the host, even api:serve, which is tagged
        // itself: the workloads replace it...
        assert!(
            !targets(&plan.resources)
                .iter()
                .any(|t| t.starts_with("api:"))
        );
        // ...every workload stands in as its service for dependents...
        assert_eq!(
            resource(&plan.resources, "web:dev").resource_deps,
            ["api", "api-worker", "web:codegen"]
        );
        // ...and they wait on their own upstream services, which are pulled in.
        assert_eq!(plan.workload_deps.get("api").unwrap(), &["db:serve"]);
        assert!(targets(&plan.resources).contains(&"db:serve"));
    }

    #[test]
    fn tagged_project_with_workloads_has_no_local_resources() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let plan = plan(&projects, &tasks, &config, &k8s("web", &["web"]));

        // web is tagged, but its server and the build step it pulled in are gone.
        assert!(
            !targets(&plan.resources)
                .iter()
                .any(|t| t.starts_with("web:"))
        );
        assert_eq!(plan.workload_deps.get("web").unwrap(), &["api:serve"]);
    }

    #[test]
    fn task_tagged_build_steps_still_run_for_containerized_projects() {
        let (projects, mut tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let codegen = tasks
            .tasks
            .get_mut("web")
            .and_then(|tasks| tasks.get_mut("codegen"))
            .unwrap();
        codegen.tags.push(config.tag.clone());
        let plan = plan(&projects, &tasks, &config, &k8s("web", &["web"]));
        let resources = targets(&plan.resources);

        assert!(resources.contains(&"web:codegen"));
        assert!(!resources.contains(&"web:dev"));
        assert!(!resources.contains(&"web:lint"));
    }

    fn render_workloads(config: &TiltExtensionConfig) -> String {
        let (mut projects, tasks) = fixture();
        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();

        api.config.project = Some(crate::query::QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "workloads": { "k8s": ["api"], "compose": ["api-cache"] } } }"#,
            )
            .unwrap(),
        });

        let Planned {
            resources,
            images,
            workloads,
            ..
        } = plan_with_images(&projects, &tasks, config, &[]);

        render(
            &RenderInput {
                images: &images,
                workloads: &workloads,
                ..RenderInput::new(&resources, &projects, "0.0.0")
            },
            config,
        )
        .unwrap()
    }

    #[test]
    fn renders_workload_decorations() {
        let output = render_workloads(&TiltExtensionConfig::default());

        assert!(output.contains(
            "k8s_resource(\n    \"api\",\n    labels = [\"api\"],\n    resource_deps = [\"db:serve\"],\n)\n"
        ));
        assert!(output.contains(
            "dc_resource(\n    \"api-cache\",\n    labels = [\"api\"],\n    resource_deps = [\"db:serve\"],\n)\n"
        ));
        // Decorations only: nothing is deployed for them.
        assert!(!output.contains("k8s_yaml"));
        assert!(!output.contains("docker_compose"));
    }

    #[test]
    fn workload_labels_can_be_left_to_project_tiltfiles() {
        let output = render_workloads(&TiltExtensionConfig {
            workload_labels: crate::config::WorkloadLabels::None,
            ..Default::default()
        });

        assert!(
            output
                .contains("k8s_resource(\n    \"api\",\n    resource_deps = [\"db:serve\"],\n)\n")
        );
        assert!(output.contains(
            "dc_resource(\n    \"api-cache\",\n    resource_deps = [\"db:serve\"],\n)\n"
        ));
    }

    #[test]
    fn can_disable_watching() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig {
            watch_inputs: false,
            ..Default::default()
        };

        for resource in plan_resources(&projects, &tasks, &config) {
            assert!(resource.watch.is_empty());
            assert!(resource.ignore.is_empty());
        }
    }

    #[test]
    fn renders_starlark() {
        let (projects, tasks) = fixture();
        let config = TiltExtensionConfig::default();
        let resources = plan_resources(&projects, &tasks, &config);
        let project_tiltfiles = ["docs/Tiltfile".to_owned()];
        let gitignore = ["**/node_modules".to_owned(), "!**/keep".to_owned()];
        let output = render(
            &RenderInput {
                project_tiltfiles: &project_tiltfiles,
                gitignore: &gitignore,
                ..RenderInput::new(&resources, &projects, "0.0.0")
            },
            &config,
        )
        .unwrap();

        assert!(output.starts_with("# Code generated by tilt_extension v0.0.0"));
        assert!(output.contains("os.path.dirname(__file__), \".\")"));
        assert!(output.contains(
            "local_resource(\n    \"web:dev\",\n    serve_cmd = [\"moon\", \"exec\", \"web:dev\", \"--upstream\", \"none\", \"--on-failure\", \"bail\", \"--ignore-ci-checks\"],\n    serve_dir = _MOON_ROOT,\n    serve_env = _MOON_ENV,\n"
        ));
        assert!(output.contains("_MOON_ENV = {\"MOON_SKIP_SYNC_WORKSPACE\": \"true\"}\n"));
        assert!(output.contains("# Project Tiltfiles\ninclude(_moon_path(\"docs/Tiltfile\"))\n"));
        assert!(output.contains(
            "    cmd = [\"moon\", \"exec\", \"web:codegen\", \"--upstream\", \"none\", \"--on-failure\", \"bail\", \"--ignore-ci-checks\"],\n    dir = _MOON_ROOT,\n    env = _MOON_ENV,\n"
        ));
        assert!(output.contains(
            "_MOON_GITIGNORE = [\n    _moon_path(\"**/node_modules\"),\n    \"!\" + _moon_path(\"**/keep\"),\n]\n"
        ));
        // Build step: moon's inputs, gitignore applied.
        assert!(output.contains(
            "    deps = [_moon_path(\"web\")],\n    ignore = [_moon_path(\"web/**\"), \"!\" + _moon_path(\"web/**/*\")] + _MOON_GITIGNORE + [_moon_path(\"web/generated/**/*\")],\n"
        ));
        // Service: upstream outputs only, gitignore not applied.
        assert!(output.contains(
            "    deps = [_moon_path(\"web/generated\")],\n    ignore = [_moon_path(\"web/generated/**\"), \"!\" + _moon_path(\"web/generated/**/*\")],\n"
        ));
        assert!(output.contains("    resource_deps = [\"api:serve\", \"web:codegen\"],\n"));
        assert!(output.contains("    labels = [\"web\"],\n"));
        assert!(!output.contains("web:lint"));
    }

    #[test]
    fn omits_include_section_without_project_tiltfiles() {
        let config = TiltExtensionConfig::default();

        assert!(
            !render(
                &RenderInput::new(&[], &QueryProjectsOutput::default(), "0.0.0"),
                &config
            )
            .unwrap()
            .contains("include(")
        );
    }

    #[test]
    fn finds_project_tiltfiles() {
        let (projects, _) = fixture();
        let config = TiltExtensionConfig::default();

        assert_eq!(
            find_project_tiltfiles(&projects, &config, |path| path != "web/Tiltfile"),
            [
                "api-client/Tiltfile",
                "api/Tiltfile",
                "db/Tiltfile",
                "docs/Tiltfile",
                "tooling/Tiltfile"
            ]
        );
        assert_eq!(
            find_project_tiltfiles(&projects, &config, |path| path == "docs/Tiltfile"),
            ["docs/Tiltfile"]
        );
    }

    #[test]
    fn never_includes_the_main_or_generated_tiltfile() {
        let mut projects = QueryProjectsOutput::default();

        for (id, source) in [("root", "."), ("app", "app")] {
            projects.projects.push(crate::query::QueryProject {
                id: id.into(),
                source: source.into(),
                ..Default::default()
            });
        }

        let config = TiltExtensionConfig::default();

        assert_eq!(
            find_project_tiltfiles(&projects, &config, |_| true),
            ["app/Tiltfile"]
        );

        let config = TiltExtensionConfig {
            tiltfile: "dev/Tiltfile".into(),
            output: "app/Tiltfile".into(),
            ..Default::default()
        };

        assert_eq!(
            find_project_tiltfiles(&projects, &config, |_| true),
            ["Tiltfile"]
        );
    }

    #[test]
    fn can_disable_project_tiltfile_includes() {
        let (projects, _) = fixture();
        let config = TiltExtensionConfig {
            include_project_tiltfiles: false,
            ..Default::default()
        };

        assert!(find_project_tiltfiles(&projects, &config, |_| true).is_empty());
    }

    #[test]
    fn renders_main_tiltfile_include() {
        let config = TiltExtensionConfig::default();

        assert!(render_main_tiltfile(&config).contains("include(\"./Tiltfile.moon\")"));

        let config = TiltExtensionConfig {
            output: ".moon/tilt/Tiltfile".into(),
            tiltfile: "dev/Tiltfile".into(),
            ..Default::default()
        };

        assert!(render_main_tiltfile(&config).contains("include(\"../.moon/tilt/Tiltfile\")"));
    }
}
