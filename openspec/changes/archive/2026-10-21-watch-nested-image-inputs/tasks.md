# Tasks

## 1. Watch every explicit input

- [x] 1.1 Share one predicate for the explicit task inputs that matter to an image (outside `.moon/`) between `image_triggers` and `plan_live_update`, so `image_triggers` watches nested inputs too (inputs inside watched project directories are still dropped as covered); verify with unit tests that a closure task's nested input (`docs/ui/README.md`) appears in both `watch` and `fall_back_on`, that an input inside a project directory isn't listed separately, and that every planned image's `fall_back_on` paths are under its `watch` paths
- [x] 1.2 Update the README's rebuild rules ("root-level files their tasks take as inputs" → explicit task input files anywhere outside `.moon/`), and verify no doc still limits rebuild triggers to root-level inputs

## 2. Verification

- [x] 2.1 Reproduce issue #14 with real moon and Tilt on a scratch copy of `examples/tilt-demo` (an image project depending on a project whose task has a nested `/docs/...` input), verify `tilt alpha tiltfile-result` fails before the fix and loads after it
- [x] 2.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), merging the deltas into `tilt-container-images` and `tilt-live-update`.
- Commit as `fix(tilt): watch every explicit task input an image falls back on`, push `fix/watch-nested-image-inputs`, and open a PR that fixes #14.
