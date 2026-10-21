# Spec Delta

## MODIFIED Requirements

### Requirement: Gitignore applies to every Tilt resource
With `globalGitignore` enabled (default), the generated file SHALL apply the gitignore patterns to all resources through `watch_settings()`, including resources defined outside the extension, while re-including the paths that must stay watched: explicit input files, paths services watch for upstream outputs, the files the generated Tiltfile is derived from, and the Tiltfiles themselves.

#### Scenario: User-defined resource
- **WHEN** a resource in the user's own Tiltfile watches `web/` and a gitignored file under `web/node_modules` changes
- **THEN** that resource does not re-run

#### Scenario: Gitignored generated Tiltfile
- **WHEN** the generated output directory is gitignored and a file in it is regenerated with different contents
- **THEN** Tilt still reloads it
