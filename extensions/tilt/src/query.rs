//! Minimal views of `moon query projects` and `moon query tasks` output.
//! Only the fields the extension needs are deserialized.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Default, Deserialize)]
pub struct QueryProjectsOutput {
    pub projects: Vec<QueryProject>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QueryProject {
    pub id: String,
    /// Workspace-relative project root (`.` for the workspace root).
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub config: QueryProjectConfig,
    /// Resolved project dependencies (explicit `dependsOn` and toolchain inferred).
    /// Serialized as `null` when there are none.
    #[serde(default)]
    pub dependencies: Option<Vec<QueryProjectDep>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QueryProjectDep {
    pub id: String,
    pub scope: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct QueryProjectConfig {
    #[serde(default)]
    pub tags: Vec<String>,
    /// The `project` block of moon.yml (title, metadata, ...).
    #[serde(default)]
    pub project: Option<QueryProjectMetadata>,
    #[serde(default)]
    pub docker: QueryProjectDocker,
}

#[derive(Debug, Default, Deserialize)]
pub struct QueryProjectDocker {
    /// `docker.file` settings for `moon docker file` (image, startTask, ...).
    #[serde(default)]
    pub file: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QueryProjectMetadata {
    /// Free-form `project.metadata`. Tilt settings live under `tilt`.
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl QueryProjectConfig {
    /// Per-project Tilt settings from `project.metadata.tilt`.
    /// moon flattens custom fields directly under `project:`, but also accept
    /// the common mistake of nesting them under `project.metadata`.
    pub fn tilt_metadata(&self) -> Option<&serde_json::Value> {
        let metadata = &self.project.as_ref()?.metadata;

        metadata
            .get("tilt")
            .or_else(|| metadata.get("metadata")?.get("tilt"))
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct QueryTasksOutput {
    /// Project ID -> task ID -> task.
    pub tasks: BTreeMap<String, BTreeMap<String, QueryTask>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct QueryTask {
    pub target: String,
    pub command: String,
    pub toolchains: Vec<String>,
    pub deps: Vec<QueryTaskDep>,
    /// Workspace-relative file paths.
    pub input_files: BTreeMap<String, serde_json::Value>,
    /// Workspace-relative globs.
    pub input_globs: BTreeMap<String, serde_json::Value>,
    pub output_files: BTreeMap<String, serde_json::Value>,
    pub output_globs: BTreeMap<String, serde_json::Value>,
    pub options: QueryTaskOptions,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct QueryTaskDep {
    pub target: String,
    /// Extra arguments moon passes when running this dependency.
    pub args: Vec<String>,
    /// Extra environment variables moon sets when running this dependency.
    pub env: BTreeMap<String, String>,
}

impl QueryTaskDep {
    /// Whether this dependency runs differently from the plain target.
    pub fn is_parameterized(&self) -> bool {
        !self.args.is_empty() || !self.env.is_empty()
    }

    /// A unique Tilt resource name for this parameterization,
    /// e.g. `web:codegen[--prod MODE=x]`.
    pub fn resource_name(&self) -> String {
        if !self.is_parameterized() {
            return self.target.clone();
        }

        let mut parts = self.args.clone();
        parts.extend(self.env.iter().map(|(key, value)| format!("{key}={value}")));

        format!("{}[{}]", self.target, parts.join(" "))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct QueryTaskOptions {
    pub interactive: bool,
    pub persistent: bool,
}
