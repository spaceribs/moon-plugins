# Tasks

## 1. Runtime-only dependencies

- [x] 1.1 Read `project.tilt.dependsOn` (through `tilt_metadata()`) and append its IDs to each project's runtime dependencies in the planner's `project_deps`, leaving the image closure unchanged; verify with unit tests that `web:dev` gets `resource_deps` on (and pulls in) `api:serve` with no moon edge, that a containerized project's workloads and dependents follow a runtime edge, that `web`'s image doesn't watch or sync `api`, and that `linkProjectDependencies: false` ignores the setting
- [x] 1.2 Fail generation when `project.tilt.dependsOn` names an unknown project or isn't a list of strings, with an error naming the project and the value; verify with unit tests for the message and a WASM integration test that `moon ext tilt -- generate` fails with it
- [x] 1.3 Document `project.tilt.dependsOn` in the extension README (service dependencies section and project settings table), including that it doesn't affect images; verify the README example matches the unit-test fixture

## 2. Verification

- [x] 2.1 With real moon and Tilt on a scratch copy of `examples/tilt-demo`, add `project.tilt.dependsOn: ["api"]` to a project with no moon edge to `api`, and verify the generated Tiltfile loads with `tilt alpha tiltfile-result` and the project's service has `resource_deps` on `api:serve`
- [x] 2.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), merging the delta into `tilt-service-dependencies`.
- Commit as `feat(tilt): declare runtime-only project dependencies with project.tilt.dependsOn`, push `feat/tilt-runtime-depends-on`, and open a PR that closes #16.
