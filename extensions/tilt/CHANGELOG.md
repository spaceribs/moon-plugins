# Changelog

## [0.3.0](https://github.com/spaceribs/moon-plugins/compare/tilt_extension-v0.2.0...tilt_extension-v0.3.0) - 2026-10-21

### Added

- *(tilt)* support native Windows for the core extension and toolchain

### Fixed

- *(tilt)* run resources with moon exec so persistent tasks run in CI
- *(tilt)* run the toolchain's installed Tilt instead of relying on PATH and proto shims
- *(tilt)* exclude tasks that depend on a task running Tilt

### Other

- cache dependency builds and moon's task cache, and lint on Linux only

## [0.2.0](https://github.com/spaceribs/moon-plugins/compare/tilt_extension-v0.1.0...tilt_extension-v0.2.0) - 2026-10-20

### Added

- *(tilt)* [**breaking**] reference user-deployed workloads instead of generating k8s manifests

### Other

- add release-plz and commit linting

## [0.1.0] - 2026-10-20

Initial release.

#### 🚀 Features

- Generate a Tiltfile from the moon task graph: tagged tasks (or tagged projects' persistent tasks) become Tilt resources that run `moon run`, with task `deps` and project `dependsOn` as `resource_deps`.
- One Tilt instance per workspace, selected by moon project ID: `moon ext tilt -- up/focus <projects>`, `tilt args -- <projects>`, or `tilt_config.json`.
- Precise change detection: Tilt watches exactly what moon hashes (input globs, negations, outputs, `.gitignore`), and services restart only when moon rebuilt something they consume (`serviceRestart`).
- Kept in sync: regenerated on `moon sync` and before `moon run`, and by a `(moon)` resource while Tilt runs. Includes every project's own `Tiltfile`.
- Custom Tera templates, extending or replacing the built-in one (`template`).
- Container images (opt-in with `project.tilt.k8s` or `project.tilt.image`): built from `moon docker file` with rebuilds scoped by the project graph. Deployed to Kubernetes with a generated Deployment, Service, port-forwards, resources, and a readiness probe, with Live Update into running containers.
- Build only some images from source: `--edit <projects>`, or `--edit-affected [--base <rev>]` using moon's change detection.
