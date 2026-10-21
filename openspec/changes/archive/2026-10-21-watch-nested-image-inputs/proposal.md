# Proposal

## Why

Issue #14: the generated Tiltfile fails to load when a project with `project.tilt.image` has, in its dependency closure, a task with a nested input outside the closure's project directories (for example `docs/ui/README.md`). Tilt rejects it with `fall_back_on paths '…' is not a child of any watched filepaths`.

The image's two file lists disagree:
- **`fall_back_on`** (Live Update's full-rebuild list) takes every explicit task input outside `.moon/`.
- **`deps`** (what's watched) takes only *root-level* explicit inputs.

The specs disagree the same way. `tilt-container-images` rebuilds for root-level task inputs, while `tilt-live-update` sends every explicit task input file to a full rebuild.

## What Changes

- An image watches every explicit input of its closure's tasks outside `.moon/`, not only root-level ones. Inputs inside a watched project directory are already covered and aren't listed twice.
- Every `fall_back_on` path is therefore watched, so the Tiltfile loads. A change to such an input rebuilds the image, which is what `fall_back_on` already meant.
- The README's rebuild rules say "explicit task input files" instead of "root-level files".

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-container-images`: "Rebuilds follow the project graph" rebuilds for any explicit task input of the closure, not only root-level ones.
- `tilt-live-update`: "Some files need a full rebuild" requires that every full-rebuild file is also watched.

## Impact

- **Code**: `image_triggers` in `extensions/tilt/src/images.rs`, plus unit tests.
- **Behavior**: images may now rebuild for nested explicit inputs outside their projects, where before the Tiltfile failed to load or the change was ignored. Released as `fix(tilt)` (patch).
- **Out of scope**: the issue's lower-priority note about lockfiles in project directories (`uv.lock`) not falling back unless they're explicit inputs. That widens which files force rebuilds, and deserves its own decision.
