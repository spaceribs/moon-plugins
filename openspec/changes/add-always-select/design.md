# Design

## Context

- `selection.tera` builds `_MOON_PROJECTS` (project ID to its selection, computed in Rust) and expands each Tiltfile arg (`to-run`) with `_MOON_PROJECTS.get(name, [name])`, then calls `config.set_enabled_resources(_moon_enabled)`. An empty list means everything, which is Tilt's behavior.
- `args.tera` drops the `(reapply)` marker before this runs.
- Workspace settings are deserialized in `config.rs` and reach templates through `TemplateContext`.

## Goals / Non-Goals

**Goals:**
- Workspace infrastructure stays up with any selection, without fake dependencies.
- The generated selection is unchanged for workspaces that don't use the setting.

**Non-Goals:**
- Validating resource names: they're defined by Tiltfiles the extension doesn't evaluate.

## Decisions

**Expand in Starlark, the same way as Tiltfile args.** The template emits `_MOON_ALWAYS = [...]` and, when `_moon_enabled` isn't empty, adds `_MOON_PROJECTS.get(name, [name])` for each entry. So an entry means exactly what it would mean as a Tiltfile arg, project IDs and resource names alike, and a project's selection (computed in Rust, with its dependencies) is reused. Expanding in Rust instead would duplicate the project-or-resource rule the template already has.

**Only non-empty selections.** Adding entries to an empty selection would turn "everything" into "only the infrastructure". The check is on the expanded list, after the reapply marker is gone.

**Emitted only when set.** With `alwaysSelect: []`, the selection section renders exactly as before, so existing generated files don't change.

**Validation.** `alwaysSelect` deserializes as a list of strings (anything else fails the extension's config parsing, as for other settings). An empty or whitespace-only entry fails generation, naming the setting, since it would silently select nothing.

**Template variable `always_select`.** Section overrides of `selection` get the list, like `projects`.

## Risks / Trade-offs

- [An entry names a resource no Tiltfile defines] → Tilt's own behavior for an unknown name in `set_enabled_resources` applies, the same as for a misspelled Tiltfile arg. Verification 2.1 records what Tilt 0.37 does, and the README states it.
- [An always-selected project depends on a selected one, or the reverse] → Duplicates in the enabled list are harmless to Tilt.

## Migration Plan

Opt-in. A workspace with `project.tilt.dependsOn` on its infrastructure in every project can list the infrastructure in `alwaysSelect` and remove those entries. Release as `feat(tilt)` (minor).
