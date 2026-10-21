use crate::config::TiltExtensionConfig;
use crate::export;
use crate::images::{self, TiltArgs};
use crate::query::{QueryProjectsOutput, QueryTasksOutput};
use crate::tiltfile;
use crate::toolchain;
use crate::watch;
use extism_pdk::*;
use moon_pdk::*;
use moon_pdk_api::*;
use serde::de::DeserializeOwned;
use std::fs;

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

#[plugin_fn]
pub fn register_extension(
    Json(_): Json<RegisterExtensionInput>,
) -> FnResult<Json<RegisterExtensionOutput>> {
    Ok(Json(RegisterExtensionOutput {
        name: "Tilt".into(),
        description: Some("Run the moon task graph as Tilt resources.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
    }))
}

#[plugin_fn]
pub fn define_extension_config() -> FnResult<Json<DefineExtensionConfigOutput>> {
    Ok(Json(DefineExtensionConfigOutput {
        schema: schematic::SchemaBuilder::build_root::<TiltExtensionConfig>(),
    }))
}

// EXECUTE (`moon ext tilt -- <command>`)

#[derive(Args)]
pub struct TiltExtensionArgs {
    #[command(subcommand)]
    pub command: Option<TiltCommand>,
}

#[derive(clap::Subcommand)]
pub enum TiltCommand {
    /// Generate Tilt resources from the moon task graph.
    Generate {
        /// Exit with an error if the generated file is out of date, instead of writing it.
        #[arg(long)]
        check: bool,

        /// Workspace-relative Tera template to render with, overriding the
        /// `template` setting.
        #[arg(long)]
        template: Option<String>,
    },
    /// Bring projects up in the single Tilt instance: adds them to a running
    /// Tilt, or starts Tilt with them. With no projects, runs everything.
    Up {
        /// moon project IDs (or Tilt resource names). Their dependencies
        /// are brought up too.
        projects: Vec<String>,

        #[command(flatten)]
        edit: EditArgs,

        /// Extra arguments for `tilt up`, after `--`.
        #[arg(last = true)]
        tilt_args: Vec<String>,
    },
    /// Run only these projects (and their dependencies) in the single Tilt
    /// instance, disabling everything else. Starts Tilt if needed.
    Focus {
        /// moon project IDs (or Tilt resource names).
        #[arg(required = true)]
        projects: Vec<String>,

        #[command(flatten)]
        edit: EditArgs,
    },
    /// Generate, then run `tilt ci` for these projects (or everything).
    Ci {
        projects: Vec<String>,

        #[command(flatten)]
        edit: EditArgs,

        /// Extra arguments for `tilt ci`, after `--`.
        #[arg(last = true)]
        tilt_args: Vec<String>,
    },
    /// Write the Kubernetes manifest that a project's removed
    /// `project.tilt.k8s` settings generated, to deploy it yourself.
    ExportK8s {
        /// moon project ID.
        project: String,

        /// Workspace-relative path to write to. Defaults to
        /// `<project>/k8s.yaml`.
        #[arg(long)]
        output: Option<String>,
    },
    /// Tear down resources created by Tilt (`tilt down`). The instance
    /// itself stops with Ctrl-C in the terminal running `up`.
    Down {
        /// Extra arguments for `tilt down`, after `--`.
        #[arg(last = true)]
        tilt_args: Vec<String>,
    },
}

/// Which projects' container images build from source. Without these flags,
/// every image builds from source (or keeps what the running Tilt uses).
#[derive(Args)]
pub struct EditArgs {
    /// Build these projects' images from source (comma-separated or repeated);
    /// other images use the version in your k8s YAML or compose file.
    #[arg(long, value_delimiter = ',')]
    pub edit: Vec<String>,

    /// Build images from source for projects affected by your changes, and
    /// everything that depends on them (`moon query projects --affected`).
    #[arg(long)]
    pub edit_affected: bool,

    /// Base revision to compare against for `--edit-affected`, e.g. `main` on
    /// a feature branch. Defaults to uncommitted changes.
    #[arg(long, requires = "edit_affected")]
    pub base: Option<String>,
}

impl EditArgs {
    fn resolve(
        &self,
        root: &VirtualPath,
        config: &TiltExtensionConfig,
    ) -> AnyResult<Option<Vec<String>>> {
        let affected = if self.edit_affected {
            Some(affected_projects(root, config, self.base.as_deref())?)
        } else {
            None
        };

        Ok(images::resolve_to_edit(&self.edit, affected))
    }
}

#[plugin_fn]
pub fn execute_extension(Json(input): Json<ExecuteExtensionInput>) -> FnResult<()> {
    let args = parse_args::<TiltExtensionArgs>(&input.args)?;
    let mut config = get_extension_config::<TiltExtensionConfig>()?;
    let root = &input.context.workspace_root;

    let Some(command) = args.command else {
        return Err(plugin_err!(
            "A command is required: <id>generate</id>, <id>up</id>, <id>focus</id>, <id>ci</id>, <id>down</id>, or <id>export-k8s</id>. For example: <shell>moon ext tilt -- up web</shell>"
        ));
    };

    match command {
        TiltCommand::Generate { check, template } => {
            if template.is_some() {
                config.template = template;
            }

            generate(root, &config, check)?;
        }
        TiltCommand::Up {
            projects,
            edit,
            tilt_args,
        } => {
            generate(root, &config, false)?;
            let to_edit = edit.resolve(root, &config)?;

            match running_args(root, &config) {
                Some(_) if projects.is_empty() && to_edit.is_none() => {
                    host_log!(
                        stdout,
                        "Tilt is already running. Use <shell>moon ext tilt -- up <project></shell> to add projects."
                    );
                }
                // Add to what's already running.
                Some(current) => {
                    let mut args = TiltArgs::parse(&current);

                    for project in projects {
                        if !args.to_run.contains(&project) {
                            args.to_run.push(project);
                        }
                    }

                    if to_edit.is_some() {
                        args.to_edit = to_edit;
                    }

                    set_args(root, &config, &args)?;
                }
                None => {
                    let args = TiltArgs {
                        to_run: projects,
                        to_edit,
                    };

                    start_tilt(root, &config, "up", &args, tilt_args)?;
                }
            }
        }
        TiltCommand::Focus { projects, edit } => {
            generate(root, &config, false)?;
            let to_edit = edit.resolve(root, &config)?;

            match running_args(root, &config) {
                Some(current) => {
                    let args = TiltArgs {
                        to_run: projects,
                        to_edit: to_edit.or(TiltArgs::parse(&current).to_edit),
                    };

                    set_args(root, &config, &args)?;
                }
                None => {
                    let args = TiltArgs {
                        to_run: projects,
                        to_edit,
                    };

                    start_tilt(root, &config, "up", &args, vec![])?;
                }
            }
        }
        TiltCommand::Ci {
            projects,
            edit,
            tilt_args,
        } => {
            generate(root, &config, false)?;

            let args = TiltArgs {
                to_run: projects,
                to_edit: edit.resolve(root, &config)?,
            };

            start_tilt(root, &config, "ci", &args, tilt_args)?;
        }
        TiltCommand::Down { tilt_args } => {
            run_tilt(root, &config, "down", tilt_args)?;
        }
        TiltCommand::ExportK8s { project, output } => {
            export_k8s(root, &config, &project, output.as_deref())?;
        }
    };

    Ok(())
}

fn query_moon<T: DeserializeOwned>(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
    kind: &str,
) -> AnyResult<T> {
    // The host traps (uncatchably) on missing commands, so check paths we can
    // see up front. Bare names are resolved from PATH by the host, and real
    // paths outside the virtual mounts aren't visible to WASI.
    let visible = ["/workspace/", "/userhome/", "/proto/", "/moon/"]
        .iter()
        .any(|prefix| config.moon_bin.starts_with(prefix));

    if visible && !VirtualPath::new(&config.moon_bin).exists() {
        return Err(anyhow!(
            "moon binary <path>{}</path> does not exist (configure <property>moonBin</property>)",
            config.moon_bin
        ));
    }

    let output = exec(
        ExecCommandInput::pipe(&config.moon_bin, ["query", kind, "--quiet"]).cwd(root.clone()),
    )?;

    if output.exit_code != 0 {
        return Err(anyhow!(
            "Failed to run <shell>moon query {kind}</shell>: {}",
            output.get_output()
        ));
    }

    Ok(serde_json::from_str(&output.stdout)?)
}

/// The rendered generated file, and what went into it.
struct Rendered {
    contents: String,
    resources: usize,
    project_tiltfiles: usize,
}

/// Query moon and render the generated file with the configured template.
fn render_generated(root: &VirtualPath, config: &TiltExtensionConfig) -> AnyResult<Rendered> {
    // Like moon's `docker.file.template`, a configured template must exist.
    let custom_template = match &config.template {
        Some(template) => {
            let path = root.join(template);

            if !path.is_file() {
                return Err(anyhow!(
                    "Tilt template <path>{template}</path> does not exist (configure <property>template</property>)"
                ));
            }

            Some(fs::read_to_string(path)?)
        }
        None => None,
    };

    let projects: QueryProjectsOutput = query_moon(root, config, "projects")?;

    if let Some(error) = export::removed_k8s_settings_error(&projects) {
        return Err(anyhow!(error));
    }
    let tasks: QueryTasksOutput = query_moon(root, config, "tasks")?;
    let project_tiltfiles =
        tiltfile::find_project_tiltfiles(&projects, config, |path| root.join(path).is_file());
    let gitignore = load_gitignore(root, &projects);
    let config_files = find_config_files(root, &projects, config);
    let planned = tiltfile::plan_with_images(&projects, &tasks, config, &config_files);
    let extension_id = get_plugin_id()?;
    let contents = tiltfile::render(
        &tiltfile::RenderInput {
            project_tiltfiles: &project_tiltfiles,
            gitignore: &gitignore,
            config_files: &config_files,
            extension_id: extension_id.as_str(),
            images: &planned.images,
            workloads: &planned.workloads,
            excluded: &planned.excluded,
            custom_template: custom_template.as_deref(),
            ..tiltfile::RenderInput::new(&planned.resources, &projects, env!("CARGO_PKG_VERSION"))
        },
        config,
    )
    .map_err(|error| {
        anyhow!(
            "Failed to render Tilt template <path>{}</path>: {error}",
            config.template.as_deref().unwrap_or("(built-in)")
        )
    })?;

    Ok(Rendered {
        contents,
        resources: planned.resources.len(),
        project_tiltfiles: project_tiltfiles.len(),
    })
}

const CONFIG_EXTENSIONS: [&str; 7] = ["yml", "yaml", "json", "jsonc", "pkl", "hcl", "toml"];

/// Workspace-relative files the generated file is derived from, which the
/// `(moon)` Tilt resource watches to regenerate: workspace config
/// (`.moon/*.<ext>` and `.moon/tasks`), each project's `moon.<ext>`, the
/// `.gitignore` files, and a custom template. New projects matched by
/// workspace globs are picked up on the next regeneration.
fn find_config_files(
    root: &VirtualPath,
    projects: &QueryProjectsOutput,
    config: &TiltExtensionConfig,
) -> Vec<String> {
    let mut files = vec![];

    if let Ok(entries) = fs::read_dir(root.join(".moon")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            if path.is_dir() && name == "tasks" {
                files.push(".moon/tasks".to_owned());
            } else if path.is_file()
                && name
                    .rsplit_once('.')
                    .is_some_and(|(_, ext)| CONFIG_EXTENSIONS.contains(&ext))
            {
                files.push(format!(".moon/{name}"));
            }
        }
    }

    let mut dirs: Vec<&str> = vec![""];
    dirs.extend(
        projects
            .projects
            .iter()
            .map(|project| project.source.as_str()),
    );

    for dir in dirs {
        let dir = dir.trim_matches('/');
        let prefix = if dir.is_empty() || dir == "." {
            String::new()
        } else {
            format!("{dir}/")
        };

        for ext in CONFIG_EXTENSIONS {
            let file = format!("{prefix}moon.{ext}");

            if root.join(&file).is_file() {
                files.push(file);
            }
        }

        let gitignore = format!("{prefix}.gitignore");

        if root.join(&gitignore).is_file() {
            files.push(gitignore);
        }
    }

    if let Some(template) = &config.template {
        files.push(template.clone());
    }

    files.sort();
    files.dedup();
    files
}

/// Translate the workspace's and each project's `.gitignore` into Tilt
/// ignore patterns. moon never hashes gitignored files as inputs, so Tilt
/// shouldn't re-run resources for them either. (Gitignore files nested deeper
/// inside projects aren't read.)
fn load_gitignore(root: &VirtualPath, projects: &QueryProjectsOutput) -> Vec<String> {
    let mut dirs: Vec<&str> = vec![""];
    dirs.extend(
        projects
            .projects
            .iter()
            .map(|project| project.source.as_str()),
    );
    dirs.sort();
    dirs.dedup();

    dirs.into_iter()
        .filter_map(|dir| {
            let contents = fs::read_to_string(root.join(dir).join(".gitignore")).ok()?;

            Some(watch::gitignore_patterns(dir, &contents))
        })
        .flatten()
        .collect()
}

/// Files written by a sync, as workspace-relative paths.
struct SyncResult {
    rendered: Rendered,
    changed: Vec<String>,
}

/// Render the generated file and write it (and a main Tiltfile if missing).
/// Files are only written when their contents change, so Tilt doesn't
/// reload needlessly.
fn sync_files(root: &VirtualPath, config: &TiltExtensionConfig) -> AnyResult<SyncResult> {
    let rendered = render_generated(root, config)?;
    let mut changed = vec![];

    if write_if_changed(&root.join(&config.output), &rendered.contents)? {
        changed.push(config.output.clone());
    }

    let main_path = root.join(&config.tiltfile);

    if config.tiltfile != config.output && !main_path.exists() {
        write_if_changed(&main_path, &tiltfile::render_main_tiltfile(config))?;
        changed.push(config.tiltfile.clone());
    }

    Ok(SyncResult { rendered, changed })
}

fn write_if_changed(path: &VirtualPath, contents: &str) -> AnyResult<bool> {
    if fs::read_to_string(path).is_ok_and(|current| current == contents) {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, contents)?;

    Ok(true)
}

fn generate(root: &VirtualPath, config: &TiltExtensionConfig, check: bool) -> AnyResult<()> {
    if check {
        let rendered = render_generated(root, config)?;

        if fs::read_to_string(root.join(&config.output)).unwrap_or_default() != rendered.contents {
            return Err(anyhow!(
                "<path>{}</path> is out of date. Run <shell>moon sync</shell> or <shell>moon ext tilt -- generate</shell>.",
                config.output
            ));
        }

        host_log!(stdout, "<path>{}</path> is up to date", config.output);

        return Ok(());
    }

    let result = sync_files(root, config)?;

    host_log!(
        stdout,
        "Generated {} Tilt resource(s) and {} project Tiltfile include(s) in <path>{}</path>",
        result.rendered.resources,
        result.rendered.project_tiltfiles,
        config.output
    );

    if result.changed.contains(&config.tiltfile) {
        host_log!(stdout, "Created <path>{}</path>", config.tiltfile);
    }

    Ok(())
}

/// Write a project's former generated manifest, refusing to overwrite.
fn export_k8s(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
    project_id: &str,
    output: Option<&str>,
) -> AnyResult<()> {
    let projects: QueryProjectsOutput = query_moon(root, config, "projects")?;
    let Some(project) = projects
        .projects
        .iter()
        .find(|project| project.id == project_id)
    else {
        return Err(anyhow!("Unknown project <id>{project_id}</id>"));
    };
    let export = export::export_k8s(project, output).map_err(|error| anyhow!(error))?;
    let path = root.join(&export.path);

    if path.exists() {
        return Err(anyhow!(
            "<path>{}</path> already exists. Remove it, or choose another file with <shell>--output</shell>.",
            export.path
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, &export.manifest)?;

    host_log!(
        stdout,
        "Wrote <path>{}</path>. {}",
        export.path,
        export.instructions
    );

    Ok(())
}

// SYNC (`moon sync`, and before `moon run`)

#[plugin_fn]
pub fn sync_workspace(Json(input): Json<SyncWorkspaceInput>) -> FnResult<Json<SyncOutput>> {
    let config = parse_extension_config::<TiltExtensionConfig>(input.extension_config)?;
    let root = &input.context.workspace_root;
    let mut output = SyncOutput::default();

    if !config.sync {
        output.skipped = true;

        return Ok(Json(output));
    }

    // A failed sync is recorded on the operation instead of failing the
    // pipeline, so a missing `moon` binary never blocks running tasks.
    let (op, result) = Operation::track_with_error("tilt-sync", || sync_files(root, &config))?;

    if let Some(error) = &op.error {
        host_log!(
            stderr,
            "<caution>Failed to sync Tilt resources:</caution> {error}"
        );
    }

    if let Some(result) = result {
        output.changed_files = result.changed.iter().map(|path| root.join(path)).collect();
    }

    output.operations.push(op);

    Ok(Json(output))
}

/// A `tilt` command that runs the version pinned by the Tilt toolchain (the
/// binary it installed through proto, which moon also pins from
/// `.prototools`), falling back to whatever `tilt` is on PATH. Tilt reads
/// `TILT_PORT`/`TILT_HOST` from the environment, so every command targets the
/// same single instance.
fn tilt_command(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
    args: Vec<String>,
    stream: bool,
) -> AnyResult<ExecCommandInput> {
    // Only read toolchain config and proto's manifest when they can matter.
    let version = (config.tilt_bin == "tilt" && has_tilt_toolchain(root))
        .then(|| load_toolchain_config::<serde_json::Value>("tilt").ok())
        .flatten()
        .and_then(|config| config.get("version")?.as_str().map(String::from));
    let manifest = version
        .as_ref()
        .and_then(|_| fs::read_to_string(format!("{}/manifest.json", toolchain::TOOL_DIR)).ok());
    let executable = toolchain::tilt_executable(
        &config.tilt_bin,
        version.as_deref(),
        manifest.as_deref(),
        toolchain::exe_name(get_host_environment()?.os.is_windows()),
    )
    .map_err(|error| anyhow!(error.message()))?;

    let mut input = if stream {
        ExecCommandInput::new(&executable.command, args)
    } else {
        ExecCommandInput::pipe(&executable.command, args)
    }
    .cwd(root.clone());

    input.paths.extend(executable.path.map(Into::into));

    Ok(input)
}

/// Whether `.moon/toolchains.*` declares a top-level `tilt` toolchain. The
/// host aborts the call (uncatchably) when loading an unconfigured
/// toolchain's config, so check the file first.
fn has_tilt_toolchain(root: &VirtualPath) -> bool {
    ["yml", "yaml", "json", "jsonc", "pkl", "hcl", "toml"]
        .iter()
        .filter_map(|ext| fs::read_to_string(root.join(format!(".moon/toolchains.{ext}"))).ok())
        .any(|contents| {
            contents.lines().any(|line| {
                let line = line.trim_end();

                // YAML/PKL/HCL/TOML keys at the top level, or a JSON key.
                ["tilt:", "tilt {", "tilt=", "tilt =", "[tilt]"]
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
                    || line.trim_start().starts_with("\"tilt\"")
            })
        })
}

fn run_tilt(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
    command: &str,
    extra_args: Vec<String>,
) -> AnyResult<()> {
    let mut args = vec![
        command.to_owned(),
        "--file".to_owned(),
        config.tiltfile.clone(),
    ];
    args.extend(extra_args);

    let output = exec(tilt_command(root, config, args, true)?)?;

    if output.exit_code != 0 {
        return Err(anyhow!(
            "<shell>{} {command}</shell> exited with code {}",
            config.tilt_bin,
            output.exit_code
        ));
    }

    Ok(())
}

/// Start Tilt (`up` or `ci`) with the given projects as Tiltfile args.
fn start_tilt(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
    command: &str,
    args: &TiltArgs,
    mut tilt_args: Vec<String>,
) -> AnyResult<()> {
    let tiltfile_args = args.to_args();

    if !tiltfile_args.is_empty() {
        tilt_args.push("--".into());
        tilt_args.extend(tiltfile_args);
    }

    run_tilt(root, config, command, tilt_args)
}

/// The Tiltfile args of the running Tilt instance, or `None` if Tilt isn't
/// running (on `TILT_PORT`, default 10350).
fn running_args(root: &VirtualPath, config: &TiltExtensionConfig) -> Option<Vec<String>> {
    let output = exec(
        tilt_command(
            root,
            config,
            vec![
                "get".into(),
                "tiltfile".into(),
                "(Tiltfile)".into(),
                "-o".into(),
                "json".into(),
            ],
            false,
        )
        .ok()?,
    )
    .ok()?;

    if output.exit_code != 0 {
        return None;
    }

    let tiltfile: serde_json::Value = serde_json::from_str(&output.stdout).ok()?;

    Some(
        tiltfile["spec"]["args"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|arg| arg.as_str().map(String::from))
            .collect(),
    )
}

/// Replace the running Tilt's selection. Tilt enables the selected
/// resources plus their dependencies, and rejects unknown names.
fn set_args(root: &VirtualPath, config: &TiltExtensionConfig, args: &TiltArgs) -> AnyResult<()> {
    let mut command = vec!["args".to_owned(), "--".to_owned()];
    command.extend(args.to_args());

    let output = exec(tilt_command(root, config, command, false)?)?;

    if output.exit_code != 0 {
        return Err(anyhow!("Failed to update Tilt: {}", output.get_output()));
    }

    let running = if args.to_run.is_empty() {
        "everything".to_owned()
    } else {
        format!(
            "<id>{}</id> (and their dependencies)",
            args.to_run.join("</id>, <id>")
        )
    };
    let editing = match &args.to_edit {
        None => "all images".to_owned(),
        Some(projects) if projects.iter().any(|p| p == "none") => "no images".to_owned(),
        Some(projects) => format!("images of <id>{}</id>", projects.join("</id>, <id>")),
    };

    host_log!(
        stdout,
        "Tilt is now running {running}, building {editing} from source"
    );

    Ok(())
}

/// Projects affected by changes (uncommitted, or against `base`), plus
/// everything that depends on them, which need rebuilding too.
fn affected_projects(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
    base: Option<&str>,
) -> AnyResult<Vec<String>> {
    let mut input = ExecCommandInput::pipe(
        &config.moon_bin,
        [
            "query",
            "projects",
            "--affected",
            "--downstream",
            "deep",
            "--quiet",
        ],
    )
    .cwd(root.clone());

    if let Some(base) = base {
        input.env.insert("MOON_BASE".into(), base.to_owned());
    }

    let output = exec(input)?;

    if output.exit_code != 0 {
        return Err(anyhow!(
            "Failed to query affected projects: {}",
            output.get_output()
        ));
    }

    let projects: QueryProjectsOutput = serde_json::from_str(&output.stdout)?;

    Ok(projects
        .projects
        .into_iter()
        .map(|project| project.id)
        .collect())
}
