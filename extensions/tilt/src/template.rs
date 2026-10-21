//! Renders the generated Tiltfiles with [Tera](https://keats.github.io/tera/),
//! like moon's `Dockerfile.tera`. Each generated file has a built-in section
//! template, registered as `moon/sections/<section>.tera` (with macros in
//! `moon/macros.tera`). A `<section>.tera` in `templateDir` replaces one, and
//! can `{% extends %}` it to override only some of its blocks.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use tera::{Context, Tera};

pub const MACROS_TEMPLATE_NAME: &str = "moon/macros.tera";
const MACROS_TEMPLATE: &str = include_str!("../templates/macros.tera");

/// Sections, in the order the entry file includes them (after `lib` and
/// `args`, which are loaded instead), with their built-in templates.
pub const SECTIONS: [(&str, &str); 7] = [
    (
        "Tiltfile",
        include_str!("../templates/sections/Tiltfile.tera"),
    ),
    ("lib", include_str!("../templates/sections/lib.tera")),
    ("args", include_str!("../templates/sections/args.tera")),
    (
        "resources",
        include_str!("../templates/sections/resources.tera"),
    ),
    ("image", include_str!("../templates/sections/image.tera")),
    (
        "workloads",
        include_str!("../templates/sections/workloads.tera"),
    ),
    (
        "selection",
        include_str!("../templates/sections/selection.tera"),
    ),
];

/// Variables available to templates.
#[derive(Debug, Default, Serialize)]
pub struct TemplateContext<'a> {
    /// Version of the extension plugin.
    pub version: &'a str,
    /// Relative path from the output directory to the workspace root.
    pub root: String,
    /// Workspace-relative path of the output directory.
    pub output_dir: &'a str,
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
    /// Workspace-relative files the generated files are derived from.
    pub config_files: &'a [String],
    /// Command that regenerates the generated files.
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
    /// Generated section files the entry file includes, relative to it.
    pub sections: Vec<String>,
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

/// The section templates: built-in ones, replaced by any overrides.
pub struct Renderer {
    tera: Tera,
    overridden: Vec<String>,
}

impl Renderer {
    /// `overrides`: section name -> template source (from `templateDir`).
    pub fn new(overrides: &BTreeMap<String, String>) -> Result<Self, String> {
        let names: Vec<&str> = SECTIONS.iter().map(|(name, _)| *name).collect();

        if let Some(unknown) = overrides
            .keys()
            .find(|name| !names.contains(&name.as_str()))
        {
            return Err(format!(
                "<file>{unknown}.tera</file> doesn't name a section. Section templates: {}",
                names
                    .iter()
                    .map(|name| format!("<file>{name}.tera</file>"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        let mut tera = Tera::default();

        tera.autoescape_on(vec![]);
        tera.register_filter("starlark", starlark_filter);
        tera.register_filter("moon_path", moon_path_filter);

        let mut templates: Vec<(String, &str)> =
            vec![(MACROS_TEMPLATE_NAME.into(), MACROS_TEMPLATE)];

        templates.extend(
            SECTIONS
                .iter()
                .map(|(name, source)| (builtin_name(name), *source)),
        );
        templates.extend(
            overrides
                .iter()
                .map(|(name, source)| (override_name(name), source.as_str())),
        );

        tera.add_raw_templates(templates).map_err(format_error)?;

        Ok(Self {
            tera,
            overridden: overrides.keys().cloned().collect(),
        })
    }

    /// Render a section's body: its override if there is one, else the
    /// built-in template. `extra` adds variables (e.g. `image`).
    pub fn render(
        &self,
        section: &str,
        context: &Context,
        extra: Option<(&str, &Value)>,
    ) -> Result<String, String> {
        let name = if self.overridden.iter().any(|name| name == section) {
            override_name(section)
        } else {
            builtin_name(section)
        };
        let rendered = match extra {
            Some((key, value)) => {
                let mut context = context.clone();
                context.insert(key, value);
                self.tera.render(&name, &context)
            }
            None => self.tera.render(&name, context),
        };

        rendered
            .map(|body| body.trim_matches('\n').to_owned())
            .map_err(format_error)
    }
}

/// The serialized template context, shared by every section.
pub fn context(context: &TemplateContext) -> Result<Context, String> {
    Context::from_serialize(context).map_err(format_error)
}

pub fn builtin_name(section: &str) -> String {
    format!("moon/sections/{section}.tera")
}

fn override_name(section: &str) -> String {
    format!("{section}.tera")
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

    fn render(section: &str, overrides: &[(&str, &str)], context: &TemplateContext) -> String {
        let overrides = overrides
            .iter()
            .map(|(name, source)| (name.to_string(), source.to_string()))
            .collect();

        Renderer::new(&overrides)
            .unwrap()
            .render(section, &self::context(context).unwrap(), None)
            .unwrap()
    }

    #[test]
    fn section_overrides_use_the_filters() {
        let context = TemplateContext {
            version: "1.2.3",
            project_tiltfiles: &["docs/Tiltfile".into()],
            ..Default::default()
        };

        assert_eq!(
            render(
                "workloads",
                &[(
                    "workloads",
                    "v{{ version }} {{ project_tiltfiles | moon_path }} {{ [1, true] | starlark }}"
                )],
                &context
            ),
            "v1.2.3 [_moon_path(\"docs/Tiltfile\")] [1, True]"
        );
    }

    #[test]
    fn section_overrides_can_extend_the_built_in_section() {
        let context = TemplateContext {
            version: "1.2.3",
            ..Default::default()
        };
        let output = render(
            "lib",
            &[(
                "lib",
                r#"{% extends "moon/sections/lib.tera" %}
{% block footer %}
EXTRA = True{% endblock footer %}"#,
            )],
            &context,
        );

        assert!(output.starts_with("_MOON_ROOT = "));
        assert!(output.contains("\nEXTRA = True\n"));
        // The exports stay outside the overridable blocks.
        assert!(output.ends_with("MOON_GITIGNORE = _MOON_GITIGNORE"));
    }

    #[test]
    fn rejects_overrides_that_name_no_section() {
        let overrides = BTreeMap::from([("resource".to_owned(), "".to_owned())]);
        let error = Renderer::new(&overrides).err().unwrap();

        assert!(error.starts_with("<file>resource.tera</file> doesn't name a section"));
        assert!(error.contains("<file>resources.tera</file>"));
    }

    #[test]
    fn readme_section_example_renders() {
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
            "workloads",
            &[(
                "workloads",
                r#"{% extends "moon/sections/workloads.tera" %}

{% block footer %}
{%- for project in projects %}{% if project.resources %}
k8s_yaml(kustomize({{ project.source ~ "/k8s" | moon_path }}))
{%- endif %}{% endfor %}
{% endblock footer %}"#,
            )],
            &context,
        );

        assert_eq!(output, "k8s_yaml(kustomize(_moon_path(\"apps/web/k8s\")))");
    }
}
