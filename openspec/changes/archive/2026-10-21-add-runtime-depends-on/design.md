# Design

## Context

The planner (`tiltfile.rs`, `plan_excluding`) builds one runtime project graph, `project_deps`, from moon's project dependencies filtered by `dependencyScopes`. Everything about service relationships reads it:
- each service's project-graph `resource_deps`, walking through projects without services
- pulling in upstream services (`includeUpstreamServices`)
- each containerized project's workload `resource_deps` (`workload_deps`)

Selection by project needs no change. Tilt enables a selected resource's `resource_deps` transitively, so new edges in `project_deps` reach selection automatically.

Image builds use a separate closure (`images.rs`, `dependency_closure`) over `project.dependencies` in all scopes. That closure drives rebuild triggers, Live Update syncs, and `fall_back_on`.

## Goals / Non-Goals

**Goals:**
- Declare runtime relationships without a moon edge, affecting only Tilt ordering and selection.

**Non-Goals:**
- Per-workload dependencies. A project-level edge applies to all of the project's services and workloads, which covers the reported case.
- A template variable for these edges. Templates see `projects[].dependencies` as today; the edges show up in resources' `resource_deps`.
- Validating cycles. Tilt rejects `resource_deps` cycles itself, and moon's own graph has the same exposure.

## Decisions

**Merge `project.tilt.dependsOn` into `project_deps` only.** The IDs are appended to the project's runtime dependencies, after the `dependencyScopes` filter, since they're runtime by definition. That gives the issue's semantics with no new code paths: ordering, inclusion, and selection all follow. The image closure isn't touched, so images don't watch or sync runtime dependencies. `linkProjectDependencies: false` disables them along with moon's edges, because they're the same kind of link.

**Unknown IDs fail generation.** This matches how misnamed workloads fail loudly, and how the removed `project.tilt.k8s` setting is reported: a validation pass over the projects before planning returns an error naming the project and the ID. Silently ignoring a typo would leave a service starting out of order with no hint why. Malformed values (not a list of strings) are reported the same way.

**Read through `tilt_metadata()`.** Like the other project settings, so both of moon's metadata spellings work.

## Risks / Trade-offs

- [A runtime edge creates a cycle with moon's edges] → Tilt reports the `resource_deps` cycle when loading, naming the resources.
- [Sync blocked by a typo] → Generation during `moon sync` records the error on the sync operation and doesn't block `moon run`, the same as other generation errors.

## Migration Plan

Nothing to migrate: the setting is opt-in. Release as `feat(tilt)` (minor).
