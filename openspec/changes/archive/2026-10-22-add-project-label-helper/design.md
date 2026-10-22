# Design

## Context

- **Labels today:** local resources from moon tasks get `labels: [project]` when planned (`tiltfile.rs`), whatever `workloadLabels` is. Declared workloads get `[project]` or `[]` through `decorate_workloads`, per `workloadLabels`. Resources a project's own Tiltfile defines get nothing. The extension can't add one later, because Tilt has no call that relabels a `local_resource`; `k8s_resource`/`dc_resource` merging only works for their own kinds.
- **`lib.star`:** shared definitions in the output directory (`.moon/tilt/lib.star` by default). It ends by exporting public names (`MOON_ROOT`, `moon_path`, …), because `load()` can't import `_` names. It's rendered from the `lib` section with the shared template context, which includes `projects`.
- **Load order:** the main Tiltfile runs `moon ext tilt -- generate` with `local()` before including the generated entry file, which then includes project Tiltfiles. So `lib.star` is current before any project Tiltfile loads it. Tilt resolves a `load()` path relative to the file doing the loading, and loads each file once, so a project Tiltfile and the generated sections share one module.

## Goals / Non-Goals

**Goals:**
- A project's other resources can carry the same label as its workloads, with the ID and the setting kept in moon.

**Non-Goals:**
- Labelling resources the extension doesn't define, without the project Tiltfile's help. Tilt doesn't allow it for local resources.
- Decorating Kubernetes or Compose entries of `project.tilt.resources`. The list is untyped, so the extension can't tell which entries `k8s_resource` would accept.
- Changing local resources' labels or the meaning of `workloadLabels`.

## Decisions

**Export the helper from `lib.star`, not a new `projects.star`.** The issue suggests a separate file. `lib.star` already exists for exactly this, exporting public names for other Tiltfiles to load, and it's rendered from the same context. A second file would add an output path to document and a section to keep in sync, and buy nothing. Workspaces that override the `lib` section keep working; their `lib.star` just lacks the helper until they extend the built-in block.

**A function, not a dict.** `moon_labels(project)` can `fail()` on an unknown ID, naming it, which a dict lookup by a project Tiltfile wouldn't do consistently (`MOON_LABELS["x"]` errors, but `.get("x", [])` hides typos). The table behind it, `_MOON_PROJECT_LABELS`, maps every project ID to its labels, so the lookup also fails for typos under `workloadLabels: "none"`.

**Labels are computed in Rust, as `projects[].labels`.** The template gets each project's labels the same way `decorate_workloads` computes them. That keeps one rule for "the project's label" and gives custom templates the value too, instead of each template re-implementing `workloadLabels`.

**Documented load path, relative to the project Tiltfile.** For the default `outputDir`, `apps/ingress/Tiltfile` loads `../../.moon/tilt/lib.star`. The README shows the depth rule and notes that a custom `outputDir` changes the path. Alternatives considered:
- Setting an environment variable or global from the generated entry file. Included files don't share globals, and an environment variable would be a hidden coupling.
- Injecting a helper into project Tiltfiles. The extension doesn't own them.

## Risks / Trade-offs

- [A project Tiltfile loaded outside the generated entry (e.g. `tilt up` in the project directory) fails if `lib.star` hasn't been generated] → The README says the helper needs the generated files, which `moon sync` and the main Tiltfile produce.
- [A custom `outputDir` or a moved project changes the relative path] → `load()` of a missing file fails loudly at load time, naming the path.
- [Workspaces that override the `lib` section don't get the helper] → It's in the built-in `preamble` block, so an override that extends the built-in section keeps it. The README notes this.

## Migration Plan

Opt-in: nothing changes until a project Tiltfile loads the helper. A workspace labelling by role can replace those labels with `moon_labels(...)`. Release as `feat(tilt)` (minor).
