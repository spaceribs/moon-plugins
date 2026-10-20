//! Renders the generated Tiltfile with [Tera](https://keats.github.io/tera/),
//! like moon's `Dockerfile.tera`. The built-in template is registered as
//! `moon/Tiltfile.moon.tera` (with macros in `moon/macros.tera`), so custom
//! templates can either replace it or `{% extends %}` it and override blocks.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use tera::{Context, Tera};

pub const DEFAULT_TEMPLATE_NAME: &str = "moon/Tiltfile.moon.tera";
pub const MACROS_TEMPLATE_NAME: &str = "moon/macros.tera";

const DEFAULT_TEMPLATE: &str = include_str!("../templates/Tiltfile.moon.tera");
const MACROS_TEMPLATE: &str = include_str!("../templates/macros.tera");
const CUSTOM_TEMPLATE_NAME: &str = "custom";

/// Variables available to templates.
#[derive(Debug, Default, Serialize)]
pub struct TemplateContext<'a> {
    /// Version of the extension plugin.
    pub version: &'a str,
    /// Relative path from the generated file's directory to the workspace root.
    pub root: String,
    /// Workspace-relative path of the generated file.
    pub output: &'a str,
    /// Tag used to select tasks and projects.
    pub tag: &'a str,
    /// Workspace-relative project Tiltfiles to include.
    pub project_tiltfiles: &'a [String],
    /// Workspace-relative ignore patterns translated from `.gitignore` files.
    pub gitignore: &'a [String],
    /// `!` patterns re-including gitignored paths that moon resources need,
    /// appended to the global `watch_settings()` ignore.
    pub gitignore_exceptions: &'a [String],
    /// Whether to apply the gitignore patterns globally (`globalGitignore`).
    pub global_gitignore: bool,
    /// Workspace-relative files the generated file is derived from.
    pub config_files: &'a [String],
    /// Command that regenerates the generated file.
    pub regenerate_cmd: Vec<String>,
    /// Whether to regenerate when `config_files` change (`watchConfig`).
    pub watch_config: bool,
    /// Whether to accept project IDs as Tilt args (`selectByProject`).
    pub select_by_project: bool,
    /// moon tasks exposed as Tilt resources.
    pub resources: &'a [crate::tiltfile::Resource],
    /// Every project in the workspace.
    pub projects: Vec<TemplateProject>,
    /// Container images built from moon's generated Dockerfiles.
    pub images: &'a [crate::images::Image],
    /// Workloads deployed by project Tiltfiles (`project.tilt.workloads`),
    /// decorated with labels and `resource_deps`.
    pub workloads: &'a [crate::workloads::DecoratedWorkload],
    /// Tasks left out because they depend on a task that runs Tilt, and would
    /// otherwise have been resources: `target`, `dependency`.
    pub excluded: &'a [crate::tiltfile::ExcludedTask],
    /// Whether any image restarts its process after a Live Update sync, so
    /// Tilt's `restart_process` extension must be loaded.
    pub restart_process: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct TemplateProject {
    pub id: String,
    pub source: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<TemplateProjectDependency>,
    /// Workspace-relative path of the project's Tiltfile, if it's included.
    pub tiltfile: Option<String>,
    /// Targets of the project's tasks exposed as resources.
    pub resources: Vec<String>,
    /// Resources enabled when the project ID is passed as a Tilt arg: its
    /// moon resources plus `project.tilt.resources` (e.g. ones defined in
    /// the project's own Tiltfile).
    pub selection: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct TemplateProjectDependency {
    pub id: String,
    pub scope: String,
}

/// Render the built-in template, or `custom` if provided.
pub fn render(context: &TemplateContext, custom: Option<&str>) -> Result<String, String> {
    let mut tera = Tera::default();

    tera.autoescape_on(vec![]);
    tera.register_filter("starlark", starlark_filter);
    tera.register_filter("moon_path", moon_path_filter);

    let mut templates = vec![
        (MACROS_TEMPLATE_NAME, MACROS_TEMPLATE),
        (DEFAULT_TEMPLATE_NAME, DEFAULT_TEMPLATE),
    ];

    if let Some(custom) = custom {
        templates.push((CUSTOM_TEMPLATE_NAME, custom));
    }

    tera.add_raw_templates(templates).map_err(format_error)?;

    let context = Context::from_serialize(context).map_err(format_error)?;
    let name = if custom.is_some() {
        CUSTOM_TEMPLATE_NAME
    } else {
        DEFAULT_TEMPLATE_NAME
    };

    let output = tera.render(name, &context).map_err(format_error)?;

    // Normalize trailing whitespace from (possibly empty) trailing blocks.
    Ok(format!("{}\n", output.trim_end()))
}

/// Tera nests the useful message (line, column, cause) in the error sources.
fn format_error(error: tera::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();

    while let Some(cause) = source {
        message.push_str(&format!("\n  {cause}"));
        source = cause.source();
    }

    message
}

/// Convert a JSON value into a Starlark literal.
pub fn to_starlark(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(number) => number.to_string(),
        // JSON string escaping is a compatible subset of Starlark's.
        Value::String(string) => serde_json::to_string(string).expect("strings serialize"),
        Value::Array(items) => {
            let items = items.iter().map(to_starlark).collect::<Vec<_>>();

            format!("[{}]", items.join(", "))
        }
        Value::Object(map) => {
            let items = map
                .iter()
                .map(|(key, value)| {
                    format!(
                        "{}: {}",
                        to_starlark(&key.as_str().into()),
                        to_starlark(value)
                    )
                })
                .collect::<Vec<_>>();

            format!("{{{}}}", items.join(", "))
        }
    }
}

/// `{{ value | starlark }}`: render any value as a Starlark literal.
fn starlark_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    Ok(Value::String(to_starlark(value)))
}

/// `{{ path | moon_path }}`: wrap workspace-relative path(s) in `_moon_path()`.
fn moon_path_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let wrap = |path: &Value| -> tera::Result<String> {
        match path {
            // Negated ignore patterns: `"!" + _moon_path("...")`
            Value::String(path) if path.starts_with('!') => Ok(format!(
                "\"!\" + _moon_path({})",
                to_starlark(&path[1..].into())
            )),
            Value::String(path) => Ok(format!(
                "_moon_path({})",
                to_starlark(&path.as_str().into())
            )),
            other => Err(tera::Error::msg(format!(
                "moon_path expects a string or list of strings, found {other}"
            ))),
        }
    };

    let rendered = match value {
        Value::Array(paths) => {
            let items = paths.iter().map(wrap).collect::<tera::Result<Vec<_>>>()?;

            format!("[{}]", items.join(", "))
        }
        path => wrap(path)?,
    };

    Ok(Value::String(rendered))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn converts_values_to_starlark() {
        assert_eq!(to_starlark(&json!(null)), "None");
        assert_eq!(to_starlark(&json!(true)), "True");
        assert_eq!(to_starlark(&json!(3)), "3");
        assert_eq!(to_starlark(&json!("a \"b\"")), "\"a \\\"b\\\"\"");
        assert_eq!(to_starlark(&json!(["a", false])), "[\"a\", False]");
        assert_eq!(to_starlark(&json!({ "k": ["v"] })), "{\"k\": [\"v\"]}");
    }

    #[test]
    fn renders_custom_templates_with_filters() {
        let context = TemplateContext {
            version: "1.2.3",
            project_tiltfiles: &["docs/Tiltfile".into()],
            ..Default::default()
        };

        let output = render(
            &context,
            Some("v{{ version }} {{ project_tiltfiles | moon_path }} {{ [1, true] | starlark }}"),
        )
        .unwrap();

        assert_eq!(output, "v1.2.3 [_moon_path(\"docs/Tiltfile\")] [1, True]\n");
    }

    #[test]
    fn custom_templates_can_extend_the_default() {
        let context = TemplateContext {
            version: "1.2.3",
            ..Default::default()
        };

        let output = render(
            &context,
            Some(
                r#"{% extends "moon/Tiltfile.moon.tera" %}
{% block footer %}k8s_yaml("deploy.yaml"){% endblock footer %}"#,
            ),
        )
        .unwrap();

        assert!(output.starts_with("# Code generated by tilt_extension v1.2.3"));
        assert!(output.contains("_MOON_ENV = "));
        assert!(output.trim_end().ends_with("k8s_yaml(\"deploy.yaml\")"));
    }

    #[test]
    fn readme_extends_example_renders() {
        let context = TemplateContext {
            version: "1.2.3",
            projects: vec![
                TemplateProject {
                    id: "web".into(),
                    source: "apps/web".into(),
                    resources: vec!["web:dev".into()],
                    ..Default::default()
                },
                TemplateProject {
                    id: "lib".into(),
                    source: "libs/lib".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let output = render(
            &context,
            Some(
                r#"{% extends "moon/Tiltfile.moon.tera" %}

{% block footer %}
{%- for project in projects %}{% if project.resources %}
k8s_yaml(kustomize({{ project.source ~ "/k8s" | moon_path }}))
{%- endif %}{% endfor %}
{% endblock footer %}"#,
            ),
        )
        .unwrap();

        assert!(output.ends_with("\nk8s_yaml(kustomize(_moon_path(\"apps/web/k8s\")))\n"));
        assert!(!output.contains("libs/lib"));
    }

    #[test]
    fn reports_template_errors_with_causes() {
        let error = render(&TemplateContext::default(), Some("{{ missing_var }}")).unwrap_err();

        assert!(error.contains("missing_var"), "{error}");
    }
}
