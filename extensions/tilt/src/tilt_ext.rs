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
        /// Exit with an error if any generated file is out of date, instead of
        /// writing them.
        #[arg(long)]
        check: bool,

        /// Workspace-relative directory of section templates, overriding the
        /// `templateDir` setting.
        #[arg(long)]
        template_dir: Option<String>,
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
        TiltCommand::Generate {
            check,
            template_dir,
        } => {
            if template_dir.is_some() {
                config.template_dir = template_dir;
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

                    set_args(root, &config, &current, &args)?;
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

                    set_args(root, &config, &current, &args)?;
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
    /// Generated files, by path relative to `outputDir`.
    files: tiltfile::GeneratedFiles,
    resources: usize,
    project_tiltfiles: usize,
    extension_id: String,
}

/// Query moon and render the generated files, with any section templates.
fn render_generated(root: &VirtualPath, config: &TiltExtensionConfig) -> AnyResult<Rendered> {
    if let Some(error) = tiltfile::removed_settings_error(config) {
        return Err(anyhow!(error));
    }

    let section_templates = load_section_templates(root, config)?;
    let projects: QueryProjectsOutput = query_moon(root, config, "projects")?;

    if let Some(error) = export::removed_k8s_settings_error(&projects) {
        return Err(anyhow!(error));
    }

    if let Some(error) = tiltfile::runtime_dependency_errors(&projects) {
        return Err(anyhow!(error));
    }

    if let Some(error) = images::live_update_settings_error(&projects) {
        return Err(anyhow!(error));
    }

    if let Some(error) =
        images::missing_dockerfiles_error(&projects, config, |path| root.join(path).is_file())
    {
        return Err(anyhow!(error));
    }
    let tasks: QueryTasksOutput = query_moon(root, config, "tasks")?;
    let project_tiltfiles =
        tiltfile::find_project_tiltfiles(&projects, config, |path| root.join(path).is_file());
    let gitignore = load_gitignore(root, &projects);
    let config_files = find_config_files(root, &projects, config);
    let planned = tiltfile::plan_with_images(&projects, &tasks, config, &config_files);

    for (project, reason) in images::unused_run_commands(&projects, &planned.images) {
        host_log!(
            stderr,
            "<caution><property>project.tilt.liveUpdate.run</property> of <id>{project}</id> won't run:</caution> {reason}"
        );
    }
    let extension_id = get_plugin_id()?;
    let files = tiltfile::render(
        &tiltfile::RenderInput {
            project_tiltfiles: &project_tiltfiles,
            gitignore: &gitignore,
            config_files: &config_files,
            extension_id: extension_id.as_str(),
            images: &planned.images,
            workloads: &planned.workloads,
            excluded: &planned.excluded,
            section_templates: Some(&section_templates),
            ..tiltfile::RenderInput::new(&planned.resources, &projects, env!("CARGO_PKG_VERSION"))
        },
        config,
    )
    .map_err(|error| {
        anyhow!(
            "Failed to render Tilt templates ({}): {error}",
            match &config.template_dir {
                Some(dir) => format!("<path>{dir}</path> and built-in"),
                None => "built-in".into(),
            }
        )
    })?;

    Ok(Rendered {
        files,
        resources: planned.resources.len(),
        project_tiltfiles: project_tiltfiles.len(),
        extension_id: extension_id.to_string(),
    })
}

/// Section templates in `templateDir`, by section name (`<section>.tera`).
/// Like moon's `docker.file.template`, a configured directory must exist.
fn load_section_templates(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
) -> AnyResult<std::collections::BTreeMap<String, String>> {
    let mut templates = std::collections::BTreeMap::new();
    let Some(dir) = &config.template_dir else {
        return Ok(templates);
    };
    let path = root.join(dir);

    if !path.is_dir() {
        return Err(anyhow!(
            "Tilt template directory <path>{dir}</path> does not exist (configure <property>templateDir</property>)"
        ));
    }

    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();

        if let Some(section) = name.strip_suffix(".tera") {
            templates.insert(section.to_owned(), fs::read_to_string(entry.path())?);
        }
    }

    Ok(templates)
}

/// `outputDir`, workspace-relative, without `./` or a trailing `/`.
fn output_dir(config: &TiltExtensionConfig) -> String {
    config
        .output_dir
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_owned()
}

const CONFIG_EXTENSIONS: [&str; 7] = ["yml", "yaml", "json", "jsonc", "pkl", "hcl", "toml"];

/// Workspace-relative files the generated file is derived from, which the
/// `(moon)` Tilt resource watches to regenerate: workspace config
/// (`.moon/*.<ext>` and `.moon/tasks`), each project's `moon.<ext>`, the
/// `.gitignore` files, and `templateDir`. New projects matched by
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

    if let Some(dir) = &config.template_dir {
        files.push(dir.trim_end_matches('/').to_owned());
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

/// Render the generated files and write them (and a main Tiltfile if
/// missing). Files are only written when their contents change, so Tilt
/// doesn't reload needlessly. Generated files that are no longer produced,
/// and a `Tiltfile.moon` left by earlier versions, are deleted.
fn sync_files(root: &VirtualPath, config: &TiltExtensionConfig) -> AnyResult<SyncResult> {
    let rendered = render_generated(root, config)?;
    let dir = output_dir(config);
    let mut changed = vec![];

    for (path, contents) in &rendered.files {
        let path = format!("{dir}/{path}");

        if write_if_changed(&root.join(&path), contents)? {
            changed.push(path);
        }
    }

    for path in stale_generated_files(root, &dir, &rendered.files) {
        fs::remove_file(root.join(&path))?;
        changed.push(path);
    }

    if is_generated(&root.join(LEGACY_OUTPUT)) {
        fs::remove_file(root.join(LEGACY_OUTPUT))?;
        changed.push(LEGACY_OUTPUT.into());
        host_log!(
            stdout,
            "Deleted <path>{LEGACY_OUTPUT}</path>, generated by an earlier version: the generated Tiltfiles are now in <path>{dir}</path>. In your main Tiltfile, replace <shell>include(\"./{LEGACY_OUTPUT}\")</shell> with:\n\n{}",
            main_tiltfile_lines(config, &rendered.extension_id)
        );
    }

    let main_path = root.join(&config.tiltfile);

    if !main_path.exists() {
        write_if_changed(
            &main_path,
            &tiltfile::render_main_tiltfile(config, &rendered.extension_id),
        )?;
        changed.push(config.tiltfile.clone());
    }

    Ok(SyncResult { rendered, changed })
}

/// Where earlier versions generated the single Tiltfile.
const LEGACY_OUTPUT: &str = "Tiltfile.moon";

/// The `local()` and `include()` lines of the main Tiltfile.
fn main_tiltfile_lines(config: &TiltExtensionConfig, extension_id: &str) -> String {
    tiltfile::render_main_tiltfile(config, extension_id)
        .lines()
        .filter(|line| line.starts_with("local(") || line.starts_with("include("))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a file was written by the extension (starts with its header).
fn is_generated(path: &VirtualPath) -> bool {
    path.is_file()
        && fs::read_to_string(path)
            .is_ok_and(|contents| contents.starts_with(tiltfile::GENERATED_HEADER))
}

/// Workspace-relative paths of files in `dir` that the extension generated
/// earlier (by their header) but doesn't produce anymore.
fn stale_generated_files(
    root: &VirtualPath,
    dir: &str,
    produced: &tiltfile::GeneratedFiles,
) -> Vec<String> {
    let mut stale = vec![];
    let mut pending = vec![String::new()];

    while let Some(relative) = pending.pop() {
        let Ok(entries) = fs::read_dir(root.join(dir).join(&relative)) else {
            continue;
        };

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            let full = root.join(dir).join(&path);

            if full.is_dir() {
                pending.push(path);
            } else if !produced.contains_key(&path) && is_generated(&full) {
                stale.push(format!("{dir}/{path}"));
            }
        }
    }

    stale.sort();
    stale
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
    let dir = output_dir(config);

    if check {
        let rendered = render_generated(root, config)?;
        let mut stale: Vec<String> = rendered
            .files
            .iter()
            .map(|(path, contents)| (format!("{dir}/{path}"), contents))
            .filter(|(path, contents)| {
                fs::read_to_string(root.join(path)).ok().as_deref() != Some(contents.as_str())
            })
            .map(|(path, _)| path)
            .collect();

        stale.extend(stale_generated_files(root, &dir, &rendered.files));

        if !stale.is_empty() {
            return Err(anyhow!(
                "Generated Tiltfiles are out of date: {}. Run <shell>moon sync</shell> or <shell>moon ext tilt -- generate</shell>.",
                stale
                    .iter()
                    .map(|path| format!("<path>{path}</path>"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        host_log!(stdout, "<path>{dir}</path> is up to date");

        return Ok(());
    }

    let result = sync_files(root, config)?;

    host_log!(
        stdout,
        "Generated {} Tilt resource(s) and {} project Tiltfile include(s) in <path>{dir}</path>",
        result.rendered.resources,
        result.rendered.project_tiltfiles,
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

    // Only report files that belong in version control: moon warns about
    // changed files "that should be committed", and the output directory
    // ignores itself.
    if let Some(result) = result {
        let output_dir = format!("{}/", output_dir(&config));

        output.changed_files = result
            .changed
            .iter()
            .filter(|path| !path.starts_with(&output_dir))
            .map(|path| root.join(path))
            .collect();
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

/// The running Tilt instance's `(Tiltfile)` object, or `None` if Tilt isn't
/// running (on `TILT_PORT`, default 10350).
fn running_tiltfile(root: &VirtualPath, config: &TiltExtensionConfig) -> Option<serde_json::Value> {
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

    serde_json::from_str(&output.stdout).ok()
}

/// The Tiltfile args of the running Tilt instance, or `None` if Tilt isn't
/// running.
fn running_args(root: &VirtualPath, config: &TiltExtensionConfig) -> Option<Vec<String>> {
    let tiltfile = running_tiltfile(root, config)?;

    Some(
        tiltfile["spec"]["args"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|arg| arg.as_str().map(String::from))
            .collect(),
    )
}

/// When the running Tilt last started loading its Tiltfile.
fn last_load_start(root: &VirtualPath, config: &TiltExtensionConfig) -> Option<String> {
    let tiltfile = running_tiltfile(root, config)?;
    let status = &tiltfile["status"];

    status["running"]["startedAt"]
        .as_str()
        .or_else(|| status["terminated"]["startedAt"].as_str())
        .map(String::from)
}

/// Wait (up to about 10 seconds) for Tilt to start a Tiltfile load after
/// `started`. Gives up quietly: the next args are set either way.
fn wait_for_load_after(root: &VirtualPath, config: &TiltExtensionConfig, started: Option<&str>) {
    for _ in 0..50 {
        if last_load_start(root, config).as_deref() != started {
            return;
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// Replace the running Tilt's selection, re-applying it even when the args
/// are unchanged. Tilt enables the selected resources plus their
/// dependencies, and rejects unknown names.
fn set_args(
    root: &VirtualPath,
    config: &TiltExtensionConfig,
    current: &[String],
    args: &TiltArgs,
) -> AnyResult<()> {
    let commands = args.set_commands(current);
    let last = commands.len() - 1;

    for (index, command) in commands.into_iter().enumerate() {
        let started = last_load_start(root, config);
        let output = exec(tilt_command(root, config, command, false)?)?;

        if output.exit_code != 0 {
            return Err(anyhow!("Failed to update Tilt: {}", output.get_output()));
        }

        // Tilt only reloads if the args differ from the last load's, so if it
        // saw the marker and the real args together it would skip both.
        if index < last {
            wait_for_load_after(root, config, started.as_deref());
        }
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
