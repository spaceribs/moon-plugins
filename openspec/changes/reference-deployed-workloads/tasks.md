# Tasks

## 1. Declared workloads in the planner

- [x] 1.1 Parse `project.tilt.workloads` (`k8s` and `compose` name lists) from moon's project query, and verify with unit tests for both kinds, an absent setting, and an empty list
- [ ] 1.2 Change the planner's containerized input from a set of project IDs to a map of project ID to declared workloads (with kind), so each containerized project's service entry is its workload names; verify with unit tests that a tagged project with a workload has no local resources, that a downstream service gets `resource_deps` on every workload of an upstream project, and that each workload depends on (and pulls in) its project's upstream services
- [ ] 1.3 Make a project ID's selection include its declared workload names, and verify with a unit test that `_MOON_PROJECTS["api"]` lists `api` and `api-worker`
- [ ] 1.4 Document `project.tilt.workloads` (declaration, kinds, containerization, matching names) in the extension README and its project-settings table, and verify the README examples match the unit-test fixtures

## 2. Workload decorations in the template

- [ ] 2.1 Add a `workloads` block to the built-in template that emits `k8s_resource(name, labels=[project], resource_deps=[...])` or `dc_resource(...)` for each declared workload, expose `workloads` in the template context, and verify with a render unit test for both kinds
- [ ] 2.2 Verify with real Tilt (`tilt alpha tiltfile-result`) that decorations merge with a project Tiltfile's own `labels`, `resource_deps`, and `port_forwards` in both evaluation orders, for a Kubernetes workload and a Compose service, and that a misnamed workload fails loading with an unknown-resource error
- [ ] 2.3 Update the README's template blocks and variables (new `workloads` block and variable) and verify the documented block order matches the template file

## 3. Decouple images from deployment and remove generation

- [ ] 3.1 Make `project.tilt.image` the only image opt-in, and remove generated Deployments and Services, the readiness probe, and `k8s_yaml`/`k8s_resource` emission from image planning and the `images` block; verify with unit tests that `project.tilt.image` builds without deploying and that generated output contains no `k8s_yaml`
- [ ] 3.2 Fail generation when a project still sets `project.tilt.k8s`, with an error naming the project, `export-k8s`, and `project.tilt.workloads`; verify with a WASM integration test (`moon ext tilt -- generate` fails with that message) and that sync records the failure without blocking
- [ ] 3.3 Rewrite the README's container section for referenced deploys (image name matching, Live Update with user manifests, the `command:` caveat for restarts) and remove `project.tilt.k8s` documentation; verify no README section still describes generated manifests

## 4. `export-k8s` migration command

- [ ] 4.1 Move the former manifest builder into an export module that renders readable YAML with a `wasm32-wasip1`-compatible serializer; verify with unit tests that the Deployment, Service, env, resources, and readiness probe match what the removed generator produced for the same settings
- [ ] 4.2 Add `moon ext tilt -- export-k8s <project>`: write `<project>/k8s.yaml`, refuse to overwrite an existing file, and print the `project.tilt.workloads` entry and Tiltfile lines (including `port_forwards` when ports were set); verify with WASM integration tests for writing, the printed lines, and the overwrite refusal
- [ ] 4.3 Add a README migration section (from `project.tilt.k8s` to `export-k8s` plus `project.tilt.workloads`), and verify following it on the example workspace produces a loadable Tiltfile

## 5. End-to-end verification

- [ ] 5.1 On the kind cluster, migrate a demo project from `project.tilt.k8s` with `export-k8s`, deploy it through its own Tiltfile and `project.tilt.workloads`, and verify the pod deploys, `moon ext tilt -- up <project>` selects it, a dependent local service waits on it, and a source edit is live-updated without a rebuild
- [ ] 5.2 Verify a Compose-deployed project with `project.tilt.workloads.compose` is containerized, selected by its ID, and linked in the graph, using real Tilt and Docker
- [ ] 5.3 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change's spec deltas validating in strict mode

## Workflow follow-up

- Archive the change once implemented and reviewed (`/opsx:archive`), merging the deltas into `openspec/specs/`.
- Commit as `feat(tilt)!: reference user-deployed workloads instead of generating k8s manifests` so release-plz makes a minor (breaking, pre-1.0) release.
- Reply on issues #1, #2, and #5 with the release, closing #2 and #5 as resolved by removal and #1 as implemented.
