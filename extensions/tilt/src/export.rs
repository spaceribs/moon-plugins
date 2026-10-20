//! `moon ext tilt -- export-k8s`: writes the Kubernetes manifest that the
//! removed `project.tilt.k8s` generator produced, so projects can migrate to
//! deploying it themselves (`project.tilt.workloads`).

use crate::query::{QueryProject, QueryProjectsOutput};
use serde::Deserialize;
use std::collections::BTreeMap;

/// A project's former `project.tilt.k8s` deployment, ready to write.
#[derive(Debug, PartialEq)]
pub struct K8sExport {
    /// Workspace-relative path to write the manifest to.
    pub path: String,
    /// The manifest, as YAML.
    pub manifest: String,
    /// What to add to the project's `moon.yml` and `Tiltfile` to deploy it.
    pub instructions: String,
}

/// Export a project's `project.tilt.k8s` settings as the manifest the removed
/// generator produced, to `output` (workspace-relative) or `<project>/k8s.yaml`.
pub fn export_k8s(project: &QueryProject, output: Option<&str>) -> Result<K8sExport, String> {
    let tilt = project.config.tilt_metadata();
    let Some(k8s) = tilt.and_then(|tilt| tilt.get("k8s")) else {
        return Err(format!(
            "Project <id>{}</id> has no <property>project.tilt.k8s</property> settings to export",
            project.id
        ));
    };
    let settings: K8sSettings = serde_json::from_value(k8s.clone())
        .map_err(|error| format!("Invalid <property>project.tilt.k8s</property>: {error}"))?;
    // The generator deployed the project's image, named after it by default.
    let image_ref = tilt
        .and_then(|tilt| tilt.get("image"))
        .and_then(|value| value.as_str())
        .unwrap_or(&project.id);
    let source = match project.source.trim_matches('/') {
        "." => "",
        source => source,
    };
    let path = match output {
        Some(output) if output.starts_with('/') => {
            return Err(format!(
                "<path>{output}</path> must be relative to the workspace root"
            ));
        }
        Some(output) => output.trim_start_matches("./").to_owned(),
        None if source.is_empty() => "k8s.yaml".to_owned(),
        None => format!("{source}/k8s.yaml"),
    };
    let tiltfile = if source.is_empty() {
        "Tiltfile".to_owned()
    } else {
        format!("{source}/Tiltfile")
    };
    let moon_yml = if source.is_empty() {
        "moon.yml".to_owned()
    } else {
        format!("{source}/moon.yml")
    };

    let mut tiltfile_lines = format!(
        "k8s_yaml({})",
        serde_json::to_string(&relative_path(source, &path)).expect("strings serialize")
    );

    if !settings.ports.is_empty() {
        let ports = settings
            .ports
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        tiltfile_lines.push_str(&format!(
            "\nk8s_resource({}, port_forwards=[{ports}])",
            serde_json::to_string(&project.id).expect("strings serialize")
        ));
    }

    let instructions = format!(
        "To deploy it, replace <property>project.tilt.k8s</property> in <path>{moon_yml}</path> with:\n\n\
         project:\n  tilt:\n    image: {image}\n    workloads: {{ k8s: [{name}] }}\n\n\
         and add to <path>{tiltfile}</path> (the generated Tiltfile includes it):\n\n{tiltfile_lines}",
        image = serde_json::to_string(image_ref).expect("strings serialize"),
        name = serde_json::to_string(&project.id).expect("strings serialize"),
    );

    Ok(K8sExport {
        path,
        manifest: k8s_manifests(&project.id, image_ref, &settings),
        instructions,
    })
}

/// `path` relative to the directory `from` (both workspace-relative).
fn relative_path(from: &str, path: &str) -> String {
    let from: Vec<&str> = from.split('/').filter(|part| !part.is_empty()).collect();
    let path: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let common = from.iter().zip(&path).take_while(|(a, b)| a == b).count();

    std::iter::repeat_n("..", from.len() - common)
        .chain(path[common..].iter().copied())
        .collect::<Vec<_>>()
        .join("/")
}

/// The error for projects that still set the removed `project.tilt.k8s`,
/// pointing to `export-k8s` and `project.tilt.workloads`. Generation fails
/// instead of ignoring the setting, which would quietly stop deploying them.
pub fn removed_k8s_settings_error(projects: &QueryProjectsOutput) -> Option<String> {
    let ids: Vec<&str> = projects
        .projects
        .iter()
        .filter(|project| {
            project
                .config
                .tilt_metadata()
                .is_some_and(|tilt| tilt.get("k8s").is_some())
        })
        .map(|project| project.id.as_str())
        .collect();

    if ids.is_empty() {
        return None;
    }

    let commands = ids
        .iter()
        .map(|id| format!("<shell>moon ext tilt -- export-k8s {id}</shell>"))
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!(
        "<property>project.tilt.k8s</property> is no longer supported (set by {}): the Tilt extension doesn't generate Kubernetes manifests anymore. \
         Run {commands} to write the manifest it generated, deploy it from the project's Tiltfile, \
         declare it in <property>project.tilt.workloads</property>, and remove <property>project.tilt.k8s</property>.",
        ids.iter()
            .map(|id| format!("<id>{id}</id>"))
            .collect::<Vec<_>>()
            .join(", "),
    ))
}

/// Per-project Kubernetes settings from `project.tilt.k8s` in moon.yml.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct K8sSettings {
    pub replicas: Option<u32>,
    /// Container ports. Each gets a Service port and a localhost port-forward.
    pub ports: Vec<u16>,
    pub env: BTreeMap<String, String>,
    /// Container `resources` (`requests`/`limits`), passed through as-is.
    /// The container runs `moon run`, so limits must leave room for moon.
    pub resources: Option<serde_json::Value>,
    /// Container `readinessProbe`, passed through as-is. Defaults to a TCP
    /// check on the first port, so Tilt reports healthy only once the app
    /// accepts connections. `false` disables it.
    pub readiness: Option<serde_json::Value>,
}

/// The Deployment and Service for a project's image, as a readable YAML
/// stream for the user to maintain.
pub fn k8s_manifests(name: &str, image_ref: &str, settings: &K8sSettings) -> String {
    k8s_documents(name, image_ref, settings)
        .iter()
        .map(|document| serde_norway::to_string(document).expect("JSON values serialize"))
        .collect::<Vec<_>>()
        .join("---\n")
}

/// The Deployment, and a Service when there are ports, exactly as the
/// removed `project.tilt.k8s` generator built them.
pub fn k8s_documents(
    name: &str,
    image_ref: &str,
    settings: &K8sSettings,
) -> Vec<serde_json::Value> {
    // Tilt sets `app.kubernetes.io/managed-by` itself.
    let labels = serde_json::json!({ "app.kubernetes.io/name": name });

    let mut container = serde_json::json!({
        "name": name,
        "image": image_ref,
    });

    if !settings.ports.is_empty() {
        container["ports"] = settings
            .ports
            .iter()
            .map(|port| serde_json::json!({ "containerPort": port }))
            .collect();
    }

    if !settings.env.is_empty() {
        container["env"] = settings
            .env
            .iter()
            .map(|(key, value)| serde_json::json!({ "name": key, "value": value }))
            .collect();
    }

    if let Some(resources) = &settings.resources {
        container["resources"] = resources.clone();
    }

    match (&settings.readiness, settings.ports.first()) {
        (Some(serde_json::Value::Bool(false)), _) => {}
        (Some(probe), _) if probe.is_object() => {
            container["readinessProbe"] = probe.clone();
        }
        (_, Some(port)) => {
            container["readinessProbe"] = serde_json::json!({
                "tcpSocket": { "port": port },
                "periodSeconds": 2,
            });
        }
        _ => {}
    }

    let deployment = serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": { "name": name, "labels": labels },
        "spec": {
            "replicas": settings.replicas.unwrap_or(1),
            "selector": { "matchLabels": { "app.kubernetes.io/name": name } },
            "template": {
                "metadata": { "labels": labels },
                "spec": { "containers": [container] },
            },
        },
    });

    let mut documents = vec![deployment];

    // A Service gives other pods a stable DNS name: http://<project>:<port>.
    if !settings.ports.is_empty() {
        let service = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Service",
            "metadata": { "name": name, "labels": labels },
            "spec": {
                "selector": { "app.kubernetes.io/name": name },
                "ports": settings
                    .ports
                    .iter()
                    .map(|port| serde_json::json!({ "name": format!("port-{port}"), "port": port, "targetPort": port }))
                    .collect::<Vec<_>>(),
            },
        });

        documents.push(service);
    }

    documents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{QueryProject, QueryProjectConfig, QueryProjectMetadata};

    fn project(id: &str, tilt: &str) -> QueryProject {
        QueryProject {
            id: id.into(),
            config: QueryProjectConfig {
                project: Some(QueryProjectMetadata {
                    metadata: serde_json::from_str(&format!(r#"{{ "tilt": {tilt} }}"#)).unwrap(),
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn errors_for_projects_still_using_k8s_settings() {
        let projects = QueryProjectsOutput {
            projects: vec![
                project("api", r#"{ "k8s": { "ports": [3001] } }"#),
                project("web", r#"{ "workloads": { "k8s": ["web"] } }"#),
                project("worker", r#"{ "k8s": {} }"#),
            ],
        };
        let error = removed_k8s_settings_error(&projects).unwrap();

        assert!(error.contains("(set by <id>api</id>, <id>worker</id>)"));
        assert!(error.contains("moon ext tilt -- export-k8s api"));
        assert!(error.contains("moon ext tilt -- export-k8s worker"));
        assert!(!error.contains("export-k8s web"));
        assert!(error.contains("project.tilt.workloads"));
    }

    fn api(tilt: &str) -> QueryProject {
        let mut api = project("api", tilt);
        api.source = "apps/api".into();
        api
    }

    #[test]
    fn exports_next_to_the_project() {
        let export = export_k8s(&api(r#"{ "k8s": { "ports": [3001] } }"#), None).unwrap();

        assert_eq!(export.path, "apps/api/k8s.yaml");
        assert_eq!(
            export.manifest,
            k8s_manifests(
                "api",
                "api",
                &K8sSettings {
                    ports: vec![3001],
                    ..Default::default()
                }
            )
        );
        assert!(export.instructions.contains(
            "project:\n  tilt:\n    image: \"api\"\n    workloads: { k8s: [\"api\"] }\n"
        ));
        assert!(
            export
                .instructions
                .contains("<path>apps/api/Tiltfile</path>")
        );
        assert!(
            export
                .instructions
                .ends_with("k8s_yaml(\"k8s.yaml\")\nk8s_resource(\"api\", port_forwards=[3001])")
        );
    }

    #[test]
    fn exports_to_a_custom_path_referenced_from_the_project() {
        let export = export_k8s(
            &api(r#"{ "k8s": {}, "image": "tilt.dev/api" }"#),
            Some("./deploy/k8s/api.yaml"),
        )
        .unwrap();

        assert_eq!(export.path, "deploy/k8s/api.yaml");
        assert!(export.manifest.contains("image: tilt.dev/api\n"));
        assert!(
            export
                .instructions
                .contains("    image: \"tilt.dev/api\"\n")
        );
        // No ports: no port-forwards.
        assert!(
            export
                .instructions
                .ends_with("k8s_yaml(\"../../deploy/k8s/api.yaml\")")
        );
    }

    #[test]
    fn export_requires_k8s_settings_and_a_relative_path() {
        assert!(
            export_k8s(&api(r#"{ "image": "api" }"#), None)
                .unwrap_err()
                .contains("no <property>project.tilt.k8s</property>")
        );
        assert!(
            export_k8s(&api(r#"{ "k8s": {} }"#), Some("/tmp/api.yaml"))
                .unwrap_err()
                .contains("relative to the workspace root")
        );
    }

    #[test]
    fn computes_relative_paths() {
        assert_eq!(relative_path("apps/api", "apps/api/k8s.yaml"), "k8s.yaml");
        assert_eq!(
            relative_path("apps/api", "apps/web/k8s.yaml"),
            "../web/k8s.yaml"
        );
        assert_eq!(relative_path("", "k8s.yaml"), "k8s.yaml");
    }

    #[test]
    fn no_error_without_k8s_settings() {
        let projects = QueryProjectsOutput {
            projects: vec![project("web", r#"{ "image": "web" }"#)],
        };

        assert_eq!(removed_k8s_settings_error(&projects), None);
    }

    #[test]
    fn generates_k8s_deployment_and_service() {
        let settings: K8sSettings = serde_json::from_str(
            r#"{
                "replicas": 2,
                "ports": [3001],
                "env": { "LOG_LEVEL": "debug" },
                "resources": { "limits": { "memory": "256Mi" } }
            }"#,
        )
        .unwrap();
        let documents = k8s_documents("api", "acme/api", &settings);

        let deployment = &documents[0];
        let container = &deployment["spec"]["template"]["spec"]["containers"][0];

        assert_eq!(deployment["kind"], "Deployment");
        assert_eq!(deployment["spec"]["replicas"], 2);
        assert_eq!(container["image"], "acme/api");
        assert_eq!(container["ports"][0]["containerPort"], 3001);
        assert_eq!(
            container["env"][0],
            serde_json::json!({ "name": "LOG_LEVEL", "value": "debug" })
        );
        assert_eq!(container["resources"]["limits"]["memory"], "256Mi");
        assert_eq!(container["readinessProbe"]["tcpSocket"]["port"], 3001);

        let service = &documents[1];

        assert_eq!(service["kind"], "Service");
        assert_eq!(service["metadata"]["name"], "api");
        assert_eq!(service["spec"]["ports"][0]["port"], 3001);
    }

    #[test]
    fn readiness_probe_can_be_customized_or_disabled() {
        let custom: K8sSettings = serde_json::from_str(
            r#"{ "ports": [80], "readiness": { "httpGet": { "path": "/health", "port": 80 } } }"#,
        )
        .unwrap();
        let disabled: K8sSettings =
            serde_json::from_str(r#"{ "ports": [80], "readiness": false }"#).unwrap();

        let container = |settings| {
            k8s_documents("a", "a", settings)[0]["spec"]["template"]["spec"]["containers"][0]
                .clone()
        };

        assert_eq!(
            container(&custom)["readinessProbe"]["httpGet"]["path"],
            "/health"
        );
        assert!(container(&disabled).get("readinessProbe").is_none());
    }

    #[test]
    fn omits_service_without_ports() {
        let documents = k8s_documents("worker", "worker", &K8sSettings::default());

        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0]["kind"], "Deployment");
        assert!(!k8s_manifests("worker", "worker", &K8sSettings::default()).contains("---"));
    }

    #[test]
    fn writes_readable_yaml_equal_to_the_generated_manifests() {
        let settings: K8sSettings = serde_json::from_str(r#"{ "ports": [3001] }"#).unwrap();
        let yaml = k8s_manifests("api", "tilt.dev/api", &settings);
        let parsed: Vec<serde_json::Value> = yaml
            .split("---\n")
            .map(|document| serde_norway::from_str(document).unwrap())
            .collect();

        assert_eq!(parsed, k8s_documents("api", "tilt.dev/api", &settings));
        assert!(yaml.starts_with("apiVersion: apps/v1\nkind: Deployment\n"));
        assert!(yaml.contains("\n---\napiVersion: v1\nkind: Service\n"));
        assert!(yaml.contains("image: tilt.dev/api\n"));
        assert!(!yaml.contains('{'));
    }
}
