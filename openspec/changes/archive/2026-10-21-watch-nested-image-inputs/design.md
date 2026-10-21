# Design

## Context

In `extensions/tilt/src/images.rs`, both lists are built from the same tasks: the tasks of every project in the image's dependency closure.
- **`image_triggers`** (an image's `deps`) watches the closure's project directories, their moon config, and task inputs that pass `!file.contains('/') && !file.starts_with(".moon")`, which means root-level files only. Then it drops paths already covered by a watched directory.
- **`plan_live_update`** (`fall_back_on`) takes the closure's moon config, plus task inputs passing `!file.starts_with(".moon")`, which means every explicit input.

Tilt requires every `fall_back_on` path to be under a `deps` path. Otherwise it fails to load the Tiltfile (issue #14).

## Goals / Non-Goals

**Goals:**
- `fall_back_on` ⊆ watched paths for every image, by construction.

**Non-Goals:**
- Treating lockfiles in project directories (e.g. `uv.lock`) as full-rebuild files when they aren't explicit inputs. That's the issue's lower-priority note, and it changes which edits force rebuilds.
- Changing which files trigger a full rebuild versus a sync.

## Decisions

**Watch every explicit task input outside `.moon/`, using the same filter as `fall_back_on`** (the issue's option 1). If a task in the image's closure takes a file as an input, the image is built from that task's world, so a change should rebuild it. That's also what `fall_back_on` already asserts. The existing "drop covered paths" pass keeps `deps` minimal: inputs inside a closure project's directory are already covered by that directory.

Alternative, the issue's option 2: filter `fall_back_on` down to watched paths. That would load, but silently ignore edits to inputs that moon hashes, which is exactly the kind of stale image the extension exists to avoid.

**Keep one shared predicate.** Both functions use one helper for "explicit inputs that matter to the image" (outside `.moon/`), so they can't drift apart again. A unit test asserts the invariant for every planned image: each `fall_back_on` path is under a `watch` path.

## Risks / Trade-offs

- [Images rebuild for more files than before] → Only for explicit inputs of tasks in the image's own closure, which moon itself treats as affecting those tasks. Before, those edits either broke the Tiltfile (with Live Update) or were ignored (without it).
- [A nested input under a gitignored path] → It's watched explicitly, as it already is for local resources. That's consistent with moon, which hashes explicit input files even when gitignored.

## Migration Plan

Nothing to migrate. Release as `fix(tilt)` (patch).
