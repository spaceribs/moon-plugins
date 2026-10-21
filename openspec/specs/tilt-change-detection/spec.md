# tilt-change-detection Specification

## Purpose
What file changes make Tilt re-run or restart a resource, so that only what moon considers affected re-runs and services keep their own live reload.

## Requirements

### Requirement: Build steps watch exactly moon's inputs
A non-persistent resource SHALL re-run only when a file moon would hash as one of the task's inputs changes: files matching its input globs (with `{a,b}` braces expanded), excluding negated inputs, the task's own outputs, and gitignored files, while always including explicit input files even when gitignored. Inputs under `.moon/` and workspace-wide globs MUST NOT be watched.

#### Scenario: Non-input file changes
- **WHEN** a file in the project directory changes that matches none of the task's input globs
- **THEN** the resource does not re-run

#### Scenario: Gitignored file changes
- **WHEN** a gitignored file matching an input glob changes (e.g. under `node_modules`)
- **THEN** the resource does not re-run

#### Scenario: Explicit gitignored input changes
- **WHEN** a task lists `.env` explicitly as an input and `.env` is gitignored
- **THEN** changing `.env` re-runs the resource

### Requirement: Gitignore patterns come from the workspace and project roots
The extension SHALL translate the `.gitignore` files at the workspace root and at each project root into Tilt ignore patterns, anchored to the directory they were found in.

#### Scenario: Project-level gitignore
- **WHEN** `apps/web/.gitignore` contains `/generated`
- **THEN** `apps/web/generated/**` is ignored

### Requirement: Services restart according to serviceRestart
A persistent resource SHALL restart based on `serviceRestart`, which a project can override with `project.tilt.serviceRestart`: `upstreamOutputs` (default) restarts when an upstream task dependency's outputs change, `inputs` restarts when the service task's own inputs change, and `never` doesn't restart on file changes.

#### Scenario: Live-reloading server source edit
- **WHEN** a service with the default policy has its source edited
- **THEN** Tilt does not restart it

#### Scenario: Upstream build output changes
- **WHEN** the service's task dependency rebuilds and rewrites its outputs
- **THEN** the service restarts

#### Scenario: Project override
- **WHEN** a project sets `project.tilt.serviceRestart: "inputs"`
- **THEN** its services restart when their own inputs change, and other projects keep the workspace setting

### Requirement: Gitignore applies to every Tilt resource
With `globalGitignore` enabled (default), the generated file SHALL apply the gitignore patterns to all resources through `watch_settings()`, including resources defined outside the extension, while re-including the paths that must stay watched: explicit input files, paths services watch for upstream outputs, the files the generated Tiltfile is derived from, and the Tiltfiles themselves.

#### Scenario: User-defined resource
- **WHEN** a resource in the user's own Tiltfile watches `web/` and a gitignored file under `web/node_modules` changes
- **THEN** that resource does not re-run

#### Scenario: Gitignored generated Tiltfile
- **WHEN** the generated output directory is gitignored and a file in it is regenerated with different contents
- **THEN** Tilt still reloads it
