# tilt-generated-tiltfile Specification

## Purpose
How the Tilt extension writes and keeps in sync the generated Tiltfile that brings up the whole workspace, alongside a main Tiltfile the user owns.

## Requirements

### Requirement: Generated file and main Tiltfile
The extension SHALL generate the Tiltfile as a directory of files, `outputDir` (default `.moon/tilt`). The directory holds an entry `Tiltfile`, shared definitions, Tiltfile args, local resources, one file per image, workload decorations, and selection. When no main Tiltfile (`tiltfile`, default `Tiltfile`) exists, the extension SHALL create one that runs `moon ext tilt -- generate` and then includes the entry file. An existing main Tiltfile SHALL be left untouched.

#### Scenario: Existing main Tiltfile
- **WHEN** the workspace already has a `Tiltfile`
- **THEN** generation leaves it untouched

#### Scenario: Fresh clone
- **WHEN** `outputDir` is gitignored, and a developer clones the workspace and runs `tilt up` with the created main Tiltfile
- **THEN** the output is generated and loaded

#### Scenario: Image setting changed
- **WHEN** only `api/moon.yml`'s image settings change
- **THEN** only `.moon/tilt/images/api.tilt` is rewritten

#### Scenario: Section order
- **WHEN** the entry file is loaded
- **THEN** project Tiltfiles load before workload decorations, and selection runs last

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

### Requirement: Not regenerated inside images
The extension SHALL skip workspace sync, generating nothing and recording no operation, when the workspace root has a `dockerManifest.json`, which `moon docker scaffold` writes to the pruned workspace it copies into images. That workspace holds only the image's project and its dependencies, so generation there would fail on other projects' missing Dockerfiles, and nothing in the image reads the output.

#### Scenario: Start task in an image
- **WHEN** an image's start task runs `moon run`, which syncs the workspace first, in a workspace with `dockerManifest.json` at its root
- **THEN** the sync is reported as skipped, with no error and no files written

### Requirement: Sync failures don't block moon
A failure while syncing SHALL be recorded on the sync operation and reported, and MUST NOT fail `moon run`, except where moon's host aborts the call (a bare `moonBin` that isn't on `PATH`).

#### Scenario: moon query fails during sync
- **WHEN** `moonBin` points to a path inside the workspace that doesn't exist
- **THEN** `moon run` still runs tasks, and the sync operation records the error

### Requirement: Regenerated while Tilt runs
With `watchConfig` enabled (default), the generated output SHALL define a `(moon)` resource that watches the files it is derived from (`.moon` config and tasks, each project's `moon.*`, `.gitignore` files, and `templateDir`) and runs `moon ext <id> -- generate` when they change. It MUST NOT run at startup.

#### Scenario: moon.yml edited while Tilt runs
- **WHEN** a project's `moon.yml` gains the `tilt` tag while Tilt runs
- **THEN** `(moon)` regenerates the output and the new resources appear

### Requirement: Staleness check
`moon ext tilt -- generate --check` SHALL fail without writing when any generated file is out of date, missing, or no longer produced, naming the file, and succeed when all are current.

#### Scenario: Stale file in CI
- **WHEN** a task was added since the output was last generated
- **THEN** `generate --check` exits with an error naming `resources.tilt`

### Requirement: Load check
`moon ext tilt -- generate --load` SHALL load the main Tiltfile as Tilt does, without a cluster or a running Tilt, after generating. When it loads, the command SHALL succeed without printing Tilt's evaluation result. When it doesn't, the command SHALL fail with Tilt's error. With `--check`, it SHALL load only when the output is current, writing nothing.

#### Scenario: Workload no Tiltfile deploys
- **WHEN** `api` declares `workloads: { k8s: ["api"] }` but no Tiltfile deploys an `api` resource, and CI runs `moon ext tilt -- generate --load`
- **THEN** the command fails with Tilt's `k8s_resource specified unknown resource "api"` error, without connecting to a cluster

#### Scenario: Tiltfile loads
- **WHEN** every declared workload is deployed and CI runs `moon ext tilt -- generate --load`
- **THEN** the command succeeds and prints no JSON

#### Scenario: Stale output with check
- **WHEN** a task was added since the output was last generated, and CI runs `moon ext tilt -- generate --check --load`
- **THEN** the command fails naming `resources.tilt`, without loading or writing anything

#### Scenario: Tilt missing
- **WHEN** neither `tiltBin` nor the Tilt toolchain's binary is available
- **THEN** `generate --load` fails with an error naming the missing binary

### Requirement: The output directory ignores and prunes itself
The extension SHALL write a `.gitignore` containing `*` into `outputDir`, and SHALL delete files there that it generated earlier but no longer produces. It MUST NOT delete files without the generated header.

#### Scenario: Image removed
- **WHEN** a project stops setting `project.tilt.image`
- **THEN** its `images/<project>.tilt` is deleted on the next generation

#### Scenario: Unrelated file in the directory
- **WHEN** the directory contains a file without the generated header
- **THEN** generation leaves it in place

### Requirement: Unchanged files are not rewritten
Generation SHALL leave every generated file whose contents didn't change untouched (contents, modification time, and inode), so including the output from a Tiltfile that regenerates it doesn't cause reload loops.

#### Scenario: Regenerate with no changes
- **WHEN** `moon ext tilt -- generate` runs twice with no configuration change in between
- **THEN** no generated file's modification time or inode changes

### Requirement: Removed single-file settings fail with migration guidance
Setting `output` or `template` MUST fail generation with an error pointing to `outputDir` or `templateDir`. A `Tiltfile.moon` left by an earlier version, recognized by its generated header, SHALL be deleted, with a message showing the main Tiltfile line to use instead.

#### Scenario: Old output setting
- **WHEN** the extension config sets `output: "Tiltfile.moon"`
- **THEN** `moon ext tilt -- generate` fails with an error naming `outputDir`

#### Scenario: Leftover generated file
- **WHEN** the workspace has a `Tiltfile.moon` with the generated header
- **THEN** generation deletes it and reports how to update the main Tiltfile
