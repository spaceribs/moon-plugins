//! Workloads a project deploys through its own Tiltfiles (Kubernetes or
//! Compose), declared by reference in `project.tilt.workloads`. The extension
//! never generates them: it only decorates them with project labels and
//! `resource_deps`, which Tilt merges with the project's own settings.

use crate::config::WorkloadLabels;
use crate::query::{QueryProject, QueryProjectsOutput};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `project.tilt.workloads`: Tilt resource names by kind.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DeclaredWorkloads {
    /// Kubernetes workloads (`k8s_yaml`, kustomize, `helm()`), decorated with
    /// `k8s_resource`.
    pub k8s: Vec<String>,
    /// Docker Compose services, decorated with `dc_resource`.
    pub compose: Vec<String>,
}

impl DeclaredWorkloads {
    pub fn is_empty(&self) -> bool {
        self.k8s.is_empty() && self.compose.is_empty()
    }

    /// Every declared resource name, Kubernetes first.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.k8s.iter().chain(&self.compose).map(String::as_str)
    }
}

/// Which Tilt function decorates a workload.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkloadKind {
    /// `k8s_resource`
    K8s,
    /// `dc_resource`
    Compose,
}

/// A declared workload with the decorations the generated Tiltfile adds.
/// Serialized as-is into the template context (`workloads`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DecoratedWorkload {
    /// Tilt resource name, as the project's Tiltfile deploys it.
    pub name: String,
    pub kind: WorkloadKind,
    pub project: String,
    /// Labels to add (the project ID, unless `workloadLabels` is `none`).
    pub labels: Vec<String>,
    /// Upstream resources from the project graph.
    pub resource_deps: Vec<String>,
}

/// The labels the extension gives a project's declared workloads, and that
/// `moon_labels(project)` returns to its Tiltfile: its ID, or none.
pub fn project_labels(project_id: &str, labels: WorkloadLabels) -> Vec<String> {
    match labels {
        WorkloadLabels::Project => vec![project_id.to_owned()],
        WorkloadLabels::None => vec![],
    }
}

/// Decorations for every declared workload, given each containerized
/// project's upstream resources (`Plan::workload_deps`).
pub fn decorate_workloads(
    projects: &QueryProjectsOutput,
    workload_deps: &BTreeMap<String, Vec<String>>,
    labels: WorkloadLabels,
) -> Vec<DecoratedWorkload> {
    projects
        .projects
        .iter()
        .flat_map(|project| {
            let workloads = declared_workloads(project);
            let kinds = workloads
                .k8s
                .into_iter()
                .map(|name| (name, WorkloadKind::K8s))
                .chain(
                    workloads
                        .compose
                        .into_iter()
                        .map(|name| (name, WorkloadKind::Compose)),
                );

            kinds
                .map(|(name, kind)| DecoratedWorkload {
                    name,
                    kind,
                    project: project.id.clone(),
                    labels: project_labels(&project.id, labels),
                    resource_deps: workload_deps.get(&project.id).cloned().unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// A project's declared workloads. An absent or malformed setting declares none.
pub fn declared_workloads(project: &QueryProject) -> DeclaredWorkloads {
    project
        .config
        .tilt_metadata()
        .and_then(|tilt| tilt.get("workloads"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{QueryProjectConfig, QueryProjectMetadata};

    fn project(tilt: &str) -> QueryProject {
        QueryProject {
            id: "api".into(),
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
    fn parses_both_kinds() {
        let workloads = declared_workloads(&project(
            r#"{ "workloads": { "k8s": ["api", "api-worker"], "compose": ["cache"] } }"#,
        ));

        assert_eq!(workloads.k8s, ["api", "api-worker"]);
        assert_eq!(workloads.compose, ["cache"]);
        assert_eq!(
            workloads.names().collect::<Vec<_>>(),
            ["api", "api-worker", "cache"]
        );
        assert!(!workloads.is_empty());
    }

    #[test]
    fn absent_setting_declares_none() {
        assert!(declared_workloads(&project("{}")).is_empty());
        assert!(declared_workloads(&QueryProject::default()).is_empty());
    }

    #[test]
    fn empty_lists_declare_none() {
        let workloads = declared_workloads(&project(r#"{ "workloads": { "k8s": [] } }"#));

        assert!(workloads.is_empty());
        assert_eq!(workloads.compose, Vec::<String>::new());
    }

    #[test]
    fn accepts_moons_nested_metadata_spelling() {
        // `tilt_metadata` also accepts `project.metadata.tilt`.
        let mut project = project("{}");
        project.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "metadata": { "tilt": { "workloads": { "compose": ["api"] } } } }"#,
            )
            .unwrap(),
        });

        assert_eq!(declared_workloads(&project).compose, ["api"]);
    }

    #[test]
    fn decorates_each_workload_with_its_projects_deps() {
        let mut db = project("{}");
        db.id = "db".into();
        let projects = QueryProjectsOutput {
            projects: vec![
                project(r#"{ "workloads": { "k8s": ["api"], "compose": ["cache"] } }"#),
                db,
            ],
        };
        let deps = BTreeMap::from([("api".to_owned(), vec!["db:serve".to_owned()])]);
        let workloads = decorate_workloads(&projects, &deps, WorkloadLabels::Project);

        assert_eq!(
            workloads,
            [
                DecoratedWorkload {
                    name: "api".into(),
                    kind: WorkloadKind::K8s,
                    project: "api".into(),
                    labels: vec!["api".into()],
                    resource_deps: vec!["db:serve".into()],
                },
                DecoratedWorkload {
                    name: "cache".into(),
                    kind: WorkloadKind::Compose,
                    project: "api".into(),
                    labels: vec!["api".into()],
                    resource_deps: vec!["db:serve".into()],
                },
            ]
        );

        let unlabelled = decorate_workloads(&projects, &deps, WorkloadLabels::None);

        assert!(unlabelled.iter().all(|workload| workload.labels.is_empty()));
        assert_eq!(unlabelled[0].resource_deps, ["db:serve"]);
    }
}
