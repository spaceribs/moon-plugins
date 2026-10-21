# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
