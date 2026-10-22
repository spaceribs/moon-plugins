# Proposal

## Why

Issue #24: with `workloadLabels: "project"` (the default), the extension labels each declared workload with its project ID, and local resources from moon tasks get it too. Resources a project's own Tiltfile defines and lists in `project.tilt.resources` (a `local_resource`, a `helm_repo`, a second Helm release) get no label.

So Tilt's UI groups don't match what a project ID selects. `moon ext tilt -- focus ingress` enables `ingress-routes`, but the UI shows it outside the `ingress` group. Tilt can't add a label to a `local_resource` after it's defined, so the label has to be set where the resource is defined. Today that means hard-coding the project ID in the project's Tiltfile.

## What Changes

- The generated `lib.star` exports `moon_labels(project)`. It returns the labels the extension gives that project's workloads: `["<project>"]` under `workloadLabels: "project"`, `[]` under `"none"`.
- A project Tiltfile can `load()` `lib.star` by a path relative to itself and pass `labels = moon_labels("ingress")`. The project ID and the label setting then stay the single source of truth.
- An unknown project ID fails Tiltfile loading, naming the ID, so a typo can't silently leave a resource ungrouped.
- Templates get each project's labels as `projects[].labels`.
- The README's Workloads section says that `workloadLabels` labels only declared workloads, and shows the helper for the project's other resources.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-workloads`: gains a requirement for labelling a project's other resources from its own Tiltfile with the same label as its workloads.

## Impact

- **Code**:
  - `extensions/tilt/templates/sections/lib.tera` (the helper and its project-label table)
  - `extensions/tilt/src/tiltfile.rs` and `template.rs` (`projects[].labels`)
  - unit tests, the golden single-file comparison, and the README
- **Behavior**: opt-in. Nothing changes for Tiltfiles that don't load the helper. Released as `feat(tilt)` (minor).
- **Out of scope**: decorating Kubernetes or Compose resources listed in `project.tilt.resources` with `k8s_resource`/`dc_resource`. The list doesn't say which kind each entry is, and decorating a local resource that way fails loading.
