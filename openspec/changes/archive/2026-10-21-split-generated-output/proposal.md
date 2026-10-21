# Proposal

## Why

Issue #17: everything is generated into one file (`output`, default `Tiltfile.moon`). Any `moon.yml` change rewrites lines in it, and some lines are several KB long, because each image's `deps`, `ignore`, and `fall_back_on` sit on one line. Committed, the diffs are effectively unreviewable. Gitignored, a fresh clone has no generated file, and nothing documents how a bare `tilt up` gets one.

The plugins are in alpha, so this replaces the single file outright instead of adding a second rendering to maintain.

## What Changes

- **BREAKING: one output layout.** The extension generates a directory of small files, `outputDir` (default `.moon/tilt`), instead of one file:
  - an entry `Tiltfile`, which runs `watch_settings` and `include()`s the sections in order
  - `lib.star` with the shared definitions
  - `args.star` with the one `config.parse()`
  - `resources.tilt`
  - `images/<project>.tilt`, one per image
  - `workloads.tilt`
  - `selection.tilt`

  A config change touches only the files for what changed. The `output` setting is removed.
- **Owned directory:**
  - `outputDir` gets a `.gitignore` containing `*`, so it ignores itself.
  - Generated files that are no longer produced (e.g. a removed project's image file) are deleted.
- **Bootstrap:** a missing main Tiltfile is created with a bootstrap that runs `moon ext tilt -- generate` before including the entry file, so a fresh clone works with a bare `tilt up`.
- **No rewrites of unchanged files** (contents, mtime, and inode). The bootstrap relies on this, so it's documented and tested.
- **`generate --check`** covers every file in `outputDir`.
- **BREAKING: per-section custom templates.** `templateDir` replaces `template` (and `--template-dir` replaces `--template`). A `<section>.tera` file there replaces that section's built-in template, and can `{% extends "moon/sections/<section>.tera" %}` to override only some of its blocks.
- **Migration:**
  - Setting `output` or `template` fails generation, with a message pointing to `outputDir` or `templateDir`.
  - An old generated `Tiltfile.moon` (recognized by its header) is deleted once, with a message showing the main Tiltfile line to use.
- **Docs:** the layout, the gitignored setup, section templates, migrating from `Tiltfile.moon`, and whether a gitignored output still needs `sync: false` in CI.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-generated-tiltfile`: the generated output becomes a directory, with the bootstrap main Tiltfile, self-ignoring and pruning, no rewrites of unchanged files, and the `output` migration. The staleness check and the `(moon)` resource cover the directory.
- `tilt-templates`: custom templates become per-section overrides in `templateDir`.
- `tilt-change-detection`: the gitignored-output scenario refers to the generated directory.

## Impact

- **Code**:
  - `config.rs`: `outputDir` and `templateDir`. `output` and `template` are removed, kept only for the migration errors.
  - `template.rs`, and the built-in template split into section templates.
  - `tiltfile.rs`: rendering the file set, and the gitignore exceptions.
  - `tilt_ext.rs`: writing, pruning, the check, the main Tiltfile, legacy cleanup, and `--template-dir`.
- **Tests**: render tests and WASM tests that read `Tiltfile.moon` move to the directory layout.
- **Users**: breaking for every workspace (the main Tiltfile's include changes), released as `feat(tilt)!` (0.x minor bump).
