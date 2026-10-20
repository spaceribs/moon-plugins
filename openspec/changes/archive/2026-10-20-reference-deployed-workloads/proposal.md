# Proposal

## Why

The Tilt extension only treats a project as containerized when it generates the project's Kubernetes manifest (`project.tilt.k8s`). Real workspaces deploy with their own manifests, Helm charts, and Compose files. Generated manifests can't express everything those need (liveness probes, init containers, Secrets, label conventions: issues #2 and #5), and projects that deploy their own lose every project-graph feature: their host tasks still run alongside the pod, their ID selects nothing, and dependents don't wait on them (issue #1). In the downstream workspace that filed these, about 10 of 18 deployed projects are in that position. Deployment definitions should stay with the user and the tools Tilt already supports, and the extension should reference them, not generate them.

## What Changes

- Projects declare the Tilt workloads their own Tiltfiles deploy, by kind and name: `project.tilt.workloads: { k8s: [...], compose: [...] }`. Kubernetes covers raw YAML, kustomize, Helm charts rendered with `helm()`, and Tilt's `helm_resource` extension.
- A project with declared workloads is containerized, as k8s mode is today: its persistent tasks, and tasks selected through the project tag or `includePersistent`, don't run as local resources (non-persistent tasks tagged individually, such as build steps, still do), its ID selects its workloads, the workloads wait on their upstream services, and downstream services wait on them.
- The extension decorates declared workloads (`k8s_resource` / `dc_resource` with `resource_deps`, and a project label unless the new `workloadLabels: none` setting turns it off) without replacing anything the project's Tiltfile sets.
- Image builds become independent of deployment: `project.tilt.image` builds an image that the user's own manifests or Compose files reference by name, with Live Update unchanged.
- **BREAKING**: `project.tilt.k8s` and generated Kubernetes manifests (Deployment, Service, readiness probe, port-forwards) are removed. Generation fails with a migration message when `project.tilt.k8s` is still present.
- A one-time `moon ext tilt -- export-k8s <project> [--output <path>]` command writes what the removed generator produced to the project, as a starting point for its own manifest.

## Capabilities

### New Capabilities
- `tilt-workloads`: declaring, by reference, the workloads a project deploys through its own Tiltfile (Kubernetes or Compose), and how those workloads join the project graph, selection, and labels.

### Modified Capabilities
- `tilt-kubernetes-deploy`: all requirements removed. Manifest generation is replaced by referenced workloads.
- `tilt-container-images`: images are opted into with `project.tilt.image` alone, and building an image no longer implies deploying it.
- `tilt-project-selection`: a project ID selects its declared workloads, not a generated Kubernetes workload.
- `tilt-live-update`: images deployed only by Compose restart their container (`restart_container()`) instead of using `restart_process`, which Tilt doesn't support for Compose.

## Impact

- **Code**: `extensions/tilt/src/images.rs` (remove `Workload`, `K8sSettings`, manifest generation; add the export helper), `tiltfile.rs` (containerized set and workload names come from `project.tilt.workloads`, supporting several per project), `query.rs` (parse workloads), `tilt_ext.rs` (export command, migration error), and the built-in template's `images` block (decorations instead of `k8s_yaml` + `k8s_resource`).
- **Configuration**: new `project.tilt.workloads` and workspace `workloadLabels`; removed `project.tilt.k8s`. Template variables change (`workload` is removed from images; a `workloads` list is added).
- **Users**: breaking for anyone using `project.tilt.k8s`, released as `feat(tilt)!` (0.x minor bump). Issues #1, #2, and #5 are resolved by this change; #3 and #4 are unrelated and stay open.
- **Docs and specs**: extension README (containers section rewritten), the five capabilities above.
