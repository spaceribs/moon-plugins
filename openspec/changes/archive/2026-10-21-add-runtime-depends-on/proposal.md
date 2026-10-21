# Proposal

## Why

Issue #16: services and workloads only wait on each other, and come up together, when moon's project graph links their projects. Runtime relationships often have no moon edge. A frontend that talks to an API usually consumes a client generated from a committed schema, so `moon ext tilt -- up web` doesn't start `api`, and workspaces put the ordering in their own Tiltfiles instead.

Adding a moon `dependsOn` edge isn't a substitute. Image builds follow every project dependency in all scopes, so the frontend's image would watch and sync the API's source.

## What Changes

- A new project setting, `project.tilt.dependsOn: ["api", ...]`, declares runtime-only dependencies on other projects.
- These edges join the same graph moon's own dependencies feed:
  - the project's services and declared workloads wait on the dependency's services and workloads
  - those are included when the project's resources are
  - selecting the project brings them up
- They don't affect image builds: an image's rebuild triggers, Live Update syncs, and fallbacks still follow moon's project graph only.
- An ID that isn't a moon project fails generation with an error naming the project and the unknown ID.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-service-dependencies`: gains a requirement for runtime-only dependencies declared in `project.tilt.dependsOn`.

## Impact

- **Code**: `extensions/tilt/src/tiltfile.rs` (the runtime project graph), a validation in generation (`tilt_ext.rs`), unit tests, and the README's project settings.
- **Behavior**: opt-in per project, so nothing changes until a project sets it. Released as `feat(tilt)` (minor).
- **Out of scope**: per-workload dependencies (the issue's "or per workload"), and exposing these edges as a separate template variable. The project-level setting covers the reported case.
