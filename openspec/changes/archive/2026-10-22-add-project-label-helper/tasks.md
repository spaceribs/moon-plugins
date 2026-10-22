# Tasks

## 1. The label helper

- [x] 1.1 Add `labels` to the template's `projects` entries, computed like `decorate_workloads` (`[id]` under `workloadLabels: "project"`, `[]` under `"none"`), and document it in the README's template variables table; verify with unit tests for both settings
- [x] 1.2 In the `lib` section's `preamble` block, render `_MOON_PROJECT_LABELS` (every project ID to its labels) and `_moon_labels(project)`, which returns a copy of the labels or calls `fail()` naming an unknown ID, and export it as `moon_labels`. Verify with render unit tests (both settings, and that every project is listed) and that the golden single-file comparison still passes
- [x] 1.3 Document the helper in the README's Workloads section: `workloadLabels` labels only declared workloads; load `lib.star` relative to the project Tiltfile (with the default `outputDir` example and the depth rule) and pass `labels = moon_labels("<project>")`; it needs the generated files; and a `lib` override must extend the built-in block to keep it. Verify the README example matches a unit-test fixture

## 2. Verification

- [x] 2.1 With real moon and Tilt 0.37 on a scratch copy of `examples/tilt-demo`, give a project a Tiltfile that defines a `local_resource` with `labels = moon_labels("<project>")`, loading `lib.star` by a relative path. Verify with `tilt alpha tiltfile-result` that the resource has the project's label, that it has none under `workloadLabels: "none"`, and that `moon_labels("<typo>")` fails loading with an error naming the ID
- [x] 2.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), merging the delta into `tilt-workloads`.
- Commit as `feat(tilt): export moon_labels from lib.star for project Tiltfiles`, push `feat/tilt-project-labels`, and open a PR that closes #24.
