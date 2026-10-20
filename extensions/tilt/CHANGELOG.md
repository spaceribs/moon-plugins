# Changelog

## Unreleased

Initial release.

#### 🚀 Features

- Generate a Tiltfile from the moon task graph: tagged tasks (or tagged projects' persistent tasks) become Tilt resources that run `moon run`, with task `deps` and project `dependsOn` as `resource_deps`.
- One Tilt instance per workspace, selected by moon project ID: `moon ext tilt -- up/focus <projects>`, `tilt args -- <projects>`, or `tilt_config.json`.
- Precise change detection: Tilt watches exactly what moon hashes (input globs, negations, outputs, `.gitignore`), and services restart only when moon rebuilt something they consume (`serviceRestart`).
- Kept in sync: regenerated on `moon sync` and before `moon run`, and by a `(moon)` resource while Tilt runs. Includes every project's own `Tiltfile`.
- Custom Tera templates, extending or replacing the built-in one (`template`).
- Container images (opt-in with `project.tilt.k8s` or `project.tilt.image`): built from `moon docker file` with rebuilds scoped by the project graph. Deployed to Kubernetes with a generated Deployment, Service, port-forwards, resources, and a readiness probe, with Live Update into running containers.
- Build only some images from source: `--edit <projects>`, or `--edit-affected [--base <rev>]` using moon's change detection.
