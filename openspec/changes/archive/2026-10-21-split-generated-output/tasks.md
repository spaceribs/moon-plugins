# Tasks

## 1. Section templates

- [x] 1.1 Split the built-in template into section templates (`moon/sections/{Tiltfile,lib,args,resources,image,workloads,selection}.tera`) with today's blocks inside the matching section, and remove the single-file template; verify with unit tests that each section renders the same Starlark as the corresponding part of today's output (captured before the refactor), adjusted only for headers and `load()` lines

## 2. Rendering the directory

- [x] 2.1 Add `outputDir` (default `.moon/tilt`) and render the file set with headers and aliased `load()` lines (one image file per image with `image` in the context; empty sections omitted); verify with unit tests for the file list, the entry file's include order, the load lines, and `selectByProject: false`
- [x] 2.2 Replace `template`/`--template` with `templateDir`/`--template-dir`: `<section>.tera` overrides a section and can extend `moon/sections/<section>.tera`; a missing directory or an unknown `.tera` name is an error; verify with unit tests (override one block of `resources`, unknown name, missing directory) and update the existing template tests
- [x] 2.3 Fail generation when `output` or `template` is set, with a message pointing to `outputDir`/`templateDir`; verify with unit tests for the messages

## 3. Writing the directory

- [x] 3.1 Write the files through `write_if_changed`, add the self-ignoring `.gitignore`, prune generated files no longer produced (by header only), report written and deleted files on the sync operation except the self-ignored output directory's own (moon warns about reported files "that should be committed"), and use `<outputDir>/**` and `templateDir/**` in the gitignore exceptions and `(moon)`'s watched files; verify with WASM integration tests for writing, pruning a removed image's file while keeping a non-generated file, and sync's changed files (moving the existing tests off `Tiltfile.moon`)
- [x] 3.2 Make `generate --check` cover every file (stale, missing, or no longer produced), naming the file; verify with WASM integration tests
- [x] 3.3 Create the bootstrap main Tiltfile when none exists, and delete a header-carrying `Tiltfile.moon` once with a message showing the bootstrap line; verify with WASM integration tests, plus one that a second `generate` leaves every file's mtime and inode unchanged

## 4. CI end-to-end

- [x] 4.1 Update `scripts/e2e-tilt.sh` and the example workspace (its main Tiltfile, `.gitignore`, and README) for `.moon/tilt`, and verify the script passes locally

## 5. Documentation

- [x] 5.1 Rewrite the extension README for the directory layout (layout table, the gitignored setup with the bootstrap, the no-rewrite guarantee, section templates and their blocks, the settings table, and a migration section from `Tiltfile.moon`/`template`); verify the layout and section names match the unit tests, and that no doc still mentions `Tiltfile.moon` except in the migration section

## 6. Verification

- [x] 6.1 With real moon and Tilt on a scratch copy of `examples/tilt-demo` (with an image and a workload), verify `tilt alpha tiltfile-result` loads the new layout, and reports the same manifests, `resource_deps`, and enabled resources for the `web` arg as the published v0.3.x single file
- [x] 6.2 Verify the bootstrap from a fresh state: with `.moon/tilt` deleted, `tilt alpha tiltfile-result` on the created main Tiltfile regenerates and loads
- [x] 6.3 Check with real moon whether the gitignored output is flagged by moon's changed-files check after sync in CI (`CI=true`), document the answer in the README's CI note, and verify the README matches
- [x] 6.4 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), merging the deltas into `tilt-generated-tiltfile`, `tilt-templates`, and `tilt-change-detection`.
- Commit as `feat(tilt)!: generate into .moon/tilt as small files, with per-section templates`, push `feat/tilt-output-dir`, and open a PR that closes #17.
