# tilt-generated-tiltfile Specification

## Purpose
How the Tilt extension writes and keeps in sync the generated Tiltfile that brings up the whole workspace, alongside a main Tiltfile the user owns.

## Requirements

### Requirement: Generated file and main Tiltfile
The extension SHALL write the generated Tiltfile to `output` (default `Tiltfile.moon`) and SHALL create the main Tiltfile (`tiltfile`, default `Tiltfile`) with an `include()` of the generated file only if no main Tiltfile exists.

#### Scenario: Existing main Tiltfile
- **WHEN** the workspace already has a `Tiltfile`
- **THEN** generation leaves it untouched

### Requirement: Project Tiltfiles are included
With `includeProjectTiltfiles` enabled (default), the generated file SHALL `include()` every project's own `Tiltfile`, excluding the main and generated Tiltfiles.

#### Scenario: Project with a Tiltfile
- **WHEN** project `docs` has `docs/Tiltfile`
- **THEN** the generated file includes it

### Requirement: Regenerated during workspace sync
With `sync` enabled (default), the extension SHALL regenerate the file during moon's workspace sync (`moon sync`, and before `moon run`), and SHALL only rewrite files whose contents changed.

#### Scenario: Nothing changed
- **WHEN** moon syncs and the regenerated contents equal the existing file
- **THEN** the file is not rewritten, so a running Tilt doesn't reload

### Requirement: Sync failures don't block moon
A failure while syncing SHALL be recorded on the sync operation and reported, and MUST NOT fail `moon run`, except where moon's host aborts the call (a bare `moonBin` that isn't on `PATH`).

#### Scenario: moon query fails during sync
- **WHEN** `moonBin` points to a path inside the workspace that doesn't exist
- **THEN** `moon run` still runs tasks, and the sync operation records the error

### Requirement: Regenerated while Tilt runs
With `watchConfig` enabled (default), the generated file SHALL define a `(moon)` resource that watches the files it is derived from (`.moon` config and tasks, each project's `moon.*`, `.gitignore` files, and a custom template) and runs `moon ext <id> -- generate` when they change. It MUST NOT run at startup.

#### Scenario: moon.yml edited while Tilt runs
- **WHEN** a project's `moon.yml` gains the `tilt` tag while Tilt runs
- **THEN** `(moon)` regenerates the file and the new resources appear

### Requirement: Staleness check
`moon ext tilt -- generate --check` SHALL fail without writing when the generated file is out of date, and succeed when it is current.

#### Scenario: Stale file in CI
- **WHEN** a task was added since the file was last generated
- **THEN** `generate --check` exits with an error
