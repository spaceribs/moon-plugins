# tilt-live-update Specification

## Purpose
How source changes reach running containers without rebuilding images, falling back to a full rebuild only for files that require one.

## Requirements

### Requirement: Source changes are synced
With `liveUpdate` enabled (default, overridable by `project.tilt.liveUpdate`), each image SHALL sync the directories of the project and its dependencies to the same paths under `imageWorkdir` (default `/app`) instead of rebuilding.

#### Scenario: Source edit
- **WHEN** a file in `api/src` changes
- **THEN** it is copied to `/app/api/src` in the running container, and the image isn't rebuilt

### Requirement: Some files need a full rebuild
Changes to moon config, root-level task inputs (e.g. lockfiles), and explicit task input files (e.g. `package.json`) SHALL trigger a full image rebuild instead of a sync. Every file that triggers a full rebuild SHALL also be watched by the image, so the generated Tiltfile always loads.

#### Scenario: moon.yml edited
- **WHEN** `api/moon.yml` changes
- **THEN** the image is rebuilt and redeployed

#### Scenario: Nested explicit input
- **WHEN** a task in the image's dependency closure takes `/docs/ui/README.md` as an input
- **THEN** `docs/ui/README.md` is both a full-rebuild file and watched by the image, and Tilt loads the Tiltfile

### Requirement: Restart apps without live reload
When the project's `serviceRestart` is `inputs` and it has a `docker.file.startTask`, the app SHALL be restarted after syncing, re-running `moon run <project>:<startTask>`: through Tilt's `restart_process` extension, or, when every workload the project declares is a Compose service, by restarting the container with `restart_container()`, since Tilt doesn't support `restart_process`'s entrypoint override for Compose. Otherwise the app SHALL be left to reload itself.

#### Scenario: Plain node server
- **WHEN** `api` sets `serviceRestart: "inputs"` and its source is synced
- **THEN** its process restarts with the new code

#### Scenario: Compose-deployed server
- **WHEN** `api` sets `serviceRestart: "inputs"` and `workloads: { compose: ["api"] }`, and its source is synced
- **THEN** the file is synced and the `api` container restarts, re-running `moon run api:serve`, without an image rebuild

### Requirement: Run a command after syncing
A project SHALL be able to set `project.tilt.liveUpdate.run` to a command (a shell string, or a list of strings run directly), run in the container after each sync as a Live Update `run()` step. It SHALL run before any restart, so it combines with `serviceRestart: inputs`, and SHALL NOT restart the process by itself. Settings under `project.tilt.liveUpdate` SHALL enable Live Update for the project, whatever the workspace `liveUpdate` says.

#### Scenario: Codegen without a restart
- **WHEN** `tests` sets `project.tilt.liveUpdate.run: ["moon", "run", "tests:codegen"]` and its source is synced
- **THEN** `moon run tests:codegen` runs in the container and the process isn't restarted

#### Scenario: Run and restart
- **WHEN** `api` sets `liveUpdate.run` and `serviceRestart: "inputs"`
- **THEN** after a sync the command runs, then the app restarts

### Requirement: Malformed Live Update settings are rejected
A `project.tilt.liveUpdate` that isn't a boolean or an object, has an unknown setting, or has an empty or non-string `run` SHALL fail generation with an error naming the project.

#### Scenario: Invalid command
- **WHEN** `web` sets `project.tilt.liveUpdate.run: []`
- **THEN** generation fails with an error naming `web`

### Requirement: Live Update can be disabled
With `liveUpdate: false` for the workspace or project, changes SHALL rebuild the image instead of syncing.

#### Scenario: Disabled for one project
- **WHEN** `web` sets `project.tilt.liveUpdate: false`
- **THEN** `web`'s source edits rebuild its image, and other projects still sync
