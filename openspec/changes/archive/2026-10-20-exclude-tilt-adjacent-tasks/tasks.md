# Tasks

## 1. Exclude tasks that follow a Tilt task

- [x] 1.1 In the planner, compute the set of tasks excluded because a dependency runs Tilt or is itself excluded for that reason (transitively), and leave them out of selection and the dependency walk, so no resource has a `resource_deps` entry for them; verify with unit tests for the `tilt-up` → `tilt-open` case in a tagged project, a tagged follow-up task, a three-task chain (`notify` → `tilt-open` → `tilt-up`), and an unrelated service in the same project that stays a resource
- [x] 1.2 Record, for each task excluded this way that selection would otherwise have exposed, the dependency that caused it, and expose the list to templates as `excluded` (`target`, `dependency`); verify with a unit test that the list holds `web:tilt-open` with `web:tilt-up` and omits Tilt-adjacent tasks nothing would have selected

## 2. Surface exclusions in the generated Tiltfile

- [x] 2.1 Render the `excluded` list as a comment in the built-in template's `resources` block (e.g. `web:tilt-open (depends on web:tilt-up, which runs Tilt)`), omitted when empty; verify with render unit tests for both cases
- [x] 2.2 Update the extension README (which tasks are never exposed, and the `excluded` template variable), and verify the documented behavior matches the unit-test fixtures

## 3. Verification

- [x] 3.1 In a scratch copy of `examples/tilt-demo`, add a tagged project with `tilt-up` (runs `tilt up`) and a persistent `tilt-open` depending on it, run `moon ext tilt -- generate` with real moon, and verify both are absent, the comment names `tilt-open`, and the file loads with `tilt alpha tiltfile-result`
- [x] 3.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), merging the delta into `openspec/specs/tilt-resources`.
- Commit as `fix(tilt): exclude tasks that depend on a task running Tilt` on a branch, and open a PR (main is protected). release-plz makes a patch release.
- After the PR merges, reply on and close issue #4.
