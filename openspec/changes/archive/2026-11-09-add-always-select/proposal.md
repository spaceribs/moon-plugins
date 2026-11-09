# Proposal

## Why

Issue #48: selection disables everything outside it (`config.set_enabled_resources`). A project selects its own resources plus its runtime dependencies' services, workloads, and `resources`. Some workspace infrastructure should run whenever anything runs, even though no project depends on it: an observability stack (Prometheus, Grafana, Loki, Tempo, an OTel collector shipping every pod's logs), or a shared ingress's dashboards. `moon ext tilt -- up api` disables all of it. The workarounds are a `project.tilt.dependsOn` on the infrastructure in every project's `moon.yml`, which is noisy, easy to forget, and claims a runtime dependency that isn't real, or overriding the `selection` section, which forks a section the extension owns.

## What Changes

- A workspace setting, `alwaysSelect` (project IDs or resource names, default `[]`). Each entry is added to every non-empty selection, as if it had been selected too: a project ID adds what that project selects (its resources, and its runtime dependencies' services, workloads, and `resources`), and any other value is a resource name.
- An empty selection keeps meaning "everything".
- A new template variable, `always_select`, for section overrides.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-project-selection`: always-selected projects and resources join every non-empty selection.

## Impact

- **Code**: `extensions/tilt/src/config.rs` (the setting and its validation), `tiltfile.rs` and `template.rs` (the template variable), `templates/sections/selection.tera`, the README's selection, workspace-settings, and template-variables sections, and `GLOSSARY.md` (always selected).
- **Behavior**: opt-in. With the default `[]`, the generated selection is unchanged. Released as `feat(tilt)` (minor).
- **Out of scope**: always-selected entries per project (a project that needs something declares it as a dependency), and selecting infrastructure only for some projects.
