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
Changes to moon config, root-level task inputs (e.g. lockfiles), and explicit task input files (e.g. `package.json`) SHALL trigger a full image rebuild instead of a sync.

#### Scenario: moon.yml edited
- **WHEN** `api/moon.yml` changes
- **THEN** the image is rebuilt and redeployed

### Requirement: Restart apps without live reload
When the project's `serviceRestart` is `inputs` and it has a `docker.file.startTask`, the process SHALL be restarted after syncing, re-running `moon run <project>:<startTask>` through Tilt's `restart_process` extension. Otherwise the app SHALL be left to reload itself.

#### Scenario: Plain node server
- **WHEN** `api` sets `serviceRestart: "inputs"` and its source is synced
- **THEN** its process restarts with the new code

### Requirement: Live Update can be disabled
With `liveUpdate: false` for the workspace or project, changes SHALL rebuild the image instead of syncing.

#### Scenario: Disabled for one project
- **WHEN** `web` sets `project.tilt.liveUpdate: false`
- **THEN** `web`'s source edits rebuild its image, and other projects still sync
