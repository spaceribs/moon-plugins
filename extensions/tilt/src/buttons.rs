//! Tilt buttons for moon tasks tagged `execTag`: each runs its task where its
//! project runs, through `moon ext tilt -- exec`. Display settings live under
//! `project.tilt.buttons.<task>`, since moon tasks have no free-form metadata.

use crate::config::TiltExtensionConfig;
use crate::query::{QueryProject, QueryProjectsOutput, QueryTasksOutput};
use crate::tiltfile::Resource;
use crate::workloads::declared_workloads;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Environment variable a button's text input sets, read by `exec` when no
/// args follow `--`. Tilt passes inputs only as environment variables.
pub const ARGS_ENV: &str = "MOON_TILT_ARGS";

/// A Tilt `cmd_button` for a task tagged `execTag`. Serialized as-is into the
/// template context (`buttons`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Button {
    /// moon target, used as the button's name.
    pub target: String,
    pub project: String,
    /// Label: `text`, else the task's description, else its ID.
    pub text: String,
    /// Material icon name, if set.
    pub icon: Option<String>,
    /// Resource the button appears on: the project's first declared
    /// workload, else its first local resource. `None` puts it in Tilt's
    /// navigation bar.
    pub resource: Option<String>,
    /// Ask for confirmation in Tilt's UI (`confirmTag`).
    pub confirm: bool,
    /// Label of the text input whose value is passed as args, if any.
    pub args: Option<String>,
    /// Name of that input, the environment variable `exec` reads.
    pub args_env: &'static str,
    /// What the button runs: `moon ext tilt -- exec <target>`, with `--yes`
    /// once Tilt has confirmed.
    pub command: Vec<String>,
}

/// A button for each task tagged `execTag`, attached to where its project
/// runs. `resources` are the planned local resources.
pub fn plan_buttons(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    resources: &[Resource],
    config: &TiltExtensionConfig,
) -> Vec<Button> {
    let mut buttons = vec![];

    for project in &projects.projects {
        let Some(project_tasks) = tasks.tasks.get(&project.id) else {
            continue;
        };
        let settings = button_settings(project);
        let resource = declared_workloads(project)
            .names()
            .next()
            .map(String::from)
            .or_else(|| {
                resources
                    .iter()
                    .find(|resource| resource.project == project.id)
                    .map(|resource| resource.target.clone())
            });

        for (task_id, task) in project_tasks {
            if !task.tags.contains(&config.exec_tag) {
                continue;
            }

            let settings = settings.get(task_id).cloned().unwrap_or_default();
            let confirm = task.tags.contains(&config.confirm_tag);
            let mut command = vec![
                config.moon_bin.clone(),
                "ext".into(),
                "tilt".into(),
                "--".into(),
                "exec".into(),
                task.target.clone(),
            ];

            if confirm {
                command.push("--yes".into());
            }

            buttons.push(Button {
                target: task.target.clone(),
                project: project.id.clone(),
                text: settings
                    .text
                    .or_else(|| {
                        task.description
                            .clone()
                            .filter(|text| !text.trim().is_empty())
                    })
                    .unwrap_or_else(|| task_id.clone()),
                icon: settings.icon,
                resource: resource.clone(),
                confirm,
                args: settings.args,
                args_env: ARGS_ENV,
                command,
            });
        }
    }

    buttons
}

/// One task's `project.tilt.buttons.<task>` settings.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ButtonSettings {
    /// Button label. Defaults to the task's description, then its ID.
    pub text: Option<String>,
    /// Material icon name.
    pub icon: Option<String>,
    /// Label of a text input whose value is passed to the task after `--`.
    pub args: Option<String>,
}

/// A project's `project.tilt.buttons`, by task ID. Errors name the entry.
fn parse_buttons(project: &QueryProject) -> Result<BTreeMap<String, ButtonSettings>, Vec<String>> {
    let Some(value) = project
        .config
        .tilt_metadata()
        .and_then(|tilt| tilt.get("buttons"))
    else {
        return Ok(BTreeMap::new());
    };
    let Some(entries) = value.as_object() else {
        return Err(vec![format!(
            "must be an object of button settings by task ID, not {value}"
        )]);
    };

    let mut buttons = BTreeMap::new();
    let mut errors = vec![];

    for (task, entry) in entries {
        let settings = match ButtonSettings::deserialize(entry) {
            Ok(settings) => settings,
            Err(error) => {
                errors.push(format!("<property>{task}</property>: {error}"));
                continue;
            }
        };

        for (key, value) in [
            ("text", &settings.text),
            ("icon", &settings.icon),
            ("args", &settings.args),
        ] {
            if value
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            {
                errors.push(format!(
                    "<property>{task}.{key}</property> must be a non-empty string"
                ));
            }
        }

        buttons.insert(task.clone(), settings);
    }

    if errors.is_empty() {
        Ok(buttons)
    } else {
        Err(errors)
    }
}

/// A project's valid button settings, by task ID. Invalid settings are
/// reported by [`button_settings_errors`] before planning.
pub fn button_settings(project: &QueryProject) -> BTreeMap<String, ButtonSettings> {
    parse_buttons(project).unwrap_or_default()
}

/// Errors for `project.tilt.buttons` settings that are malformed, or that
/// name a task the project doesn't have or that lacks `execTag`, so a typo
/// fails generation instead of silently not configuring a button.
pub fn button_settings_errors(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
) -> Option<String> {
    let mut errors = vec![];

    for project in &projects.projects {
        let setting = format!(
            "<property>project.tilt.buttons</property> of <id>{}</id>",
            project.id
        );
        let buttons = match parse_buttons(project) {
            Ok(buttons) => buttons,
            Err(problems) => {
                errors.extend(
                    problems
                        .into_iter()
                        .map(|problem| format!("{setting} {problem}")),
                );
                continue;
            }
        };
        let project_tasks = tasks.tasks.get(&project.id);

        for task_id in buttons.keys() {
            match project_tasks.and_then(|tasks| tasks.get(task_id)) {
                None => errors.push(format!(
                    "{setting} configures <property>{task_id}</property>, which isn't a task of the project"
                )),
                Some(task) if !task.tags.contains(&config.exec_tag) => errors.push(format!(
                    "{setting} configures <property>{task_id}</property>, which isn't tagged <id>{}</id>",
                    config.exec_tag
                )),
                Some(_) => {}
            }
        }
    }

    (!errors.is_empty()).then(|| errors.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueryProjectMetadata;

    fn fixture() -> (QueryProjectsOutput, QueryTasksOutput) {
        let projects =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/projects.json"))
                .unwrap();
        let tasks =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/tasks.json")).unwrap();

        (projects, tasks)
    }

    /// The fixture with `api:build` tagged `tilt-exec` and `api`'s
    /// `project.tilt.buttons` set to `buttons`.
    fn api_with_buttons(buttons: &str) -> (QueryProjectsOutput, QueryTasksOutput) {
        let (mut projects, mut tasks) = fixture();
        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();

        api.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(&format!(r#"{{ "tilt": {{ "buttons": {buttons} }} }}"#))
                .unwrap(),
        });
        tasks
            .tasks
            .get_mut("api")
            .unwrap()
            .get_mut("build")
            .unwrap()
            .tags = vec!["tilt-exec".into()];

        (projects, tasks)
    }

    fn errors(buttons: &str) -> Option<String> {
        let (projects, tasks) = api_with_buttons(buttons);

        button_settings_errors(&projects, &tasks, &TiltExtensionConfig::default())
    }

    /// Adds `tags` to `project:task`.
    fn tag(tasks: &mut QueryTasksOutput, project: &str, task: &str, tags: &[&str]) {
        let task = tasks.tasks.get_mut(project).unwrap().get_mut(task).unwrap();

        task.tags.extend(tags.iter().map(|tag| tag.to_string()));
    }

    fn set_tilt(projects: &mut QueryProjectsOutput, id: &str, tilt: &str) {
        projects
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .unwrap()
            .config
            .project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(&format!(r#"{{ "tilt": {tilt} }}"#)).unwrap(),
        });
    }

    fn planned(projects: &QueryProjectsOutput, tasks: &QueryTasksOutput) -> Vec<Button> {
        crate::tiltfile::plan_with_images(projects, tasks, &TiltExtensionConfig::default(), &[])
            .buttons
    }

    #[test]
    fn plans_a_button_per_tagged_task() {
        let (projects, mut tasks) = fixture();

        assert!(planned(&projects, &tasks).is_empty());

        tag(&mut tasks, "web", "lint", &["tilt-exec"]);
        let resources = crate::tiltfile::plan_resources(&projects, &tasks, &Default::default());

        assert_eq!(
            planned(&projects, &tasks),
            [Button {
                target: "web:lint".into(),
                project: "web".into(),
                text: "lint".into(),
                icon: None,
                resource: Some("web:codegen".into()),
                confirm: false,
                args: None,
                args_env: ARGS_ENV,
                command: ["moon", "ext", "tilt", "--", "exec", "web:lint"]
                    .map(String::from)
                    .into(),
            }]
        );
        // The tag doesn't expose the task as a resource.
        assert!(
            !resources
                .iter()
                .any(|resource| resource.target == "web:lint")
        );
    }

    #[test]
    fn buttons_attach_to_the_first_workload_then_resource_then_the_nav_bar() {
        let (mut projects, mut tasks) = fixture();

        tag(&mut tasks, "api", "build", &["tilt-exec"]);
        tag(&mut tasks, "api-client", "build", &["tilt-exec"]);
        set_tilt(
            &mut projects,
            "api",
            r#"{ "workloads": { "compose": ["api-cache"], "k8s": ["api", "api-worker"] } }"#,
        );
        let buttons = planned(&projects, &tasks);
        let resource = |target: &str| {
            buttons
                .iter()
                .find(|button| button.target == target)
                .unwrap()
                .resource
                .clone()
        };

        // Kubernetes first, in declared order.
        assert_eq!(resource("api:build"), Some("api".into()));
        // Nothing of api-client's runs in Tilt.
        assert_eq!(resource("api-client:build"), None);
    }

    #[test]
    fn button_labels_icons_args_and_confirmation() {
        let (mut projects, mut tasks) = fixture();

        tag(&mut tasks, "web", "lint", &["tilt-exec", "tilt-confirm"]);
        tag(&mut tasks, "web", "codegen", &["tilt-exec"]);
        tasks
            .tasks
            .get_mut("web")
            .unwrap()
            .get_mut("codegen")
            .unwrap()
            .description = Some("Generate the API client".into());
        set_tilt(
            &mut projects,
            "web",
            r#"{ "buttons": { "lint": { "text": "Lint", "icon": "check", "args": "Flags" } } }"#,
        );
        let buttons = planned(&projects, &tasks);
        let button = |target: &str| {
            buttons
                .iter()
                .find(|button| button.target == target)
                .unwrap()
        };

        assert_eq!(button("web:codegen").text, "Generate the API client");
        assert!(!button("web:codegen").confirm);

        let lint = button("web:lint");

        assert_eq!(lint.text, "Lint");
        assert_eq!(lint.icon.as_deref(), Some("check"));
        assert_eq!(lint.args.as_deref(), Some("Flags"));
        assert!(lint.confirm);
        assert_eq!(lint.command.last().unwrap(), "--yes");
    }

    #[test]
    fn readme_example() {
        let (mut projects, mut tasks) = fixture();

        set_tilt(
            &mut projects,
            "api",
            r#"{ "workloads": { "k8s": ["api"] }, "buttons": { "db-migrate": { "icon": "storage", "args": "Flags" } } }"#,
        );
        let api = tasks.tasks.get_mut("api").unwrap();

        for (id, description, tags) in [
            (
                "db-migrate",
                Some("Apply database migrations"),
                &["tilt-exec"][..],
            ),
            ("db-reset", None, &["tilt-exec", "tilt-confirm"][..]),
        ] {
            api.insert(
                id.into(),
                crate::query::QueryTask {
                    target: format!("api:{id}"),
                    tags: tags.iter().map(|tag| tag.to_string()).collect(),
                    description: description.map(String::from),
                    ..Default::default()
                },
            );
        }

        assert_eq!(
            button_settings_errors(&projects, &tasks, &TiltExtensionConfig::default()),
            None
        );

        let buttons = planned(&projects, &tasks);
        let summary: Vec<_> = buttons
            .iter()
            .map(|button| {
                (
                    button.text.as_str(),
                    button.resource.as_deref(),
                    button.icon.as_deref(),
                    button.args.as_deref(),
                    button.confirm,
                )
            })
            .collect();

        assert_eq!(
            summary,
            [
                (
                    "Apply database migrations",
                    Some("api"),
                    Some("storage"),
                    Some("Flags"),
                    false
                ),
                ("db-reset", Some("api"), None, None, true),
            ]
        );
    }

    #[test]
    fn parses_button_settings() {
        let (projects, tasks) = api_with_buttons(
            r#"{ "build": { "text": "Build", "icon": "build", "args": "Flags" } }"#,
        );
        let api = projects.projects.iter().find(|p| p.id == "api").unwrap();

        assert_eq!(
            button_settings(api)["build"],
            ButtonSettings {
                text: Some("Build".into()),
                icon: Some("build".into()),
                args: Some("Flags".into()),
            }
        );
        assert_eq!(
            button_settings_errors(&projects, &tasks, &TiltExtensionConfig::default()),
            None
        );
        assert_eq!(errors("{}"), None);
        assert_eq!(errors(r#"{ "build": {} }"#), None);
        assert_eq!(
            button_settings_errors(&fixture().0, &fixture().1, &Default::default()),
            None
        );
    }

    #[test]
    fn reports_malformed_button_settings() {
        assert_eq!(
            errors(r#""build""#).unwrap(),
            "<property>project.tilt.buttons</property> of <id>api</id> must be an object of button settings by task ID, not \"build\""
        );

        let error = errors(r#"{ "build": { "confirm": true } }"#).unwrap();
        assert!(
            error.contains("of <id>api</id> <property>build</property>: unknown field `confirm`"),
            "{error}"
        );

        let error = errors(r#"{ "build": "Build" }"#).unwrap();
        assert!(
            error.contains("<property>build</property>: invalid type: string"),
            "{error}"
        );

        let error = errors(r#"{ "build": { "icon": 5 } }"#).unwrap();
        assert!(
            error.contains("<property>build</property>: invalid type: integer"),
            "{error}"
        );

        assert_eq!(
            errors(r#"{ "build": { "text": " " } }"#).unwrap(),
            "<property>project.tilt.buttons</property> of <id>api</id> <property>build.text</property> must be a non-empty string"
        );
    }

    #[test]
    fn reports_buttons_for_missing_or_untagged_tasks() {
        assert_eq!(
            errors(r#"{ "deploy": {} }"#).unwrap(),
            "<property>project.tilt.buttons</property> of <id>api</id> configures <property>deploy</property>, which isn't a task of the project"
        );
        assert_eq!(
            errors(r#"{ "serve": {} }"#).unwrap(),
            "<property>project.tilt.buttons</property> of <id>api</id> configures <property>serve</property>, which isn't tagged <id>tilt-exec</id>"
        );

        // Every problem is listed.
        assert_eq!(
            errors(r#"{ "deploy": {}, "serve": {} }"#)
                .unwrap()
                .lines()
                .count(),
            2
        );

        // The tag is configurable.
        let (projects, tasks) = api_with_buttons(r#"{ "build": {} }"#);
        let config = TiltExtensionConfig {
            exec_tag: "button".into(),
            ..Default::default()
        };

        assert!(
            button_settings_errors(&projects, &tasks, &config)
                .unwrap()
                .contains("isn't tagged <id>button</id>")
        );
    }
}
