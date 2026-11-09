# Tasks

## 1. The setting

- [ ] 1.1 Add the workspace setting `alwaysSelect` (list of strings, default `[]`) to `config.rs`, and a validation error, reported before planning, for an empty or whitespace-only entry, naming the setting. Verify with unit tests that the default deserializes, a list is read, and an empty entry fails
- [ ] 1.2 Pass `always_select` to the template context and render, in `selection.tera`, `_MOON_ALWAYS` and its expansion into a non-empty `_moon_enabled`, only when the list isn't empty. Verify with render unit tests that the default output is unchanged and that a set list renders the expansion, and with a Starlark-level test through the golden comparison if one covers selection
- [ ] 1.3 Document `alwaysSelect` in the extension README (the selection section, with the observability example and what an unknown resource name does, plus the workspace-settings and template-variables tables) and add **always selected** to `GLOSSARY.md`'s selection table. Verify the README example matches a unit-test fixture

## 2. Verification

- [ ] 2.1 With real moon 2.6.2 and Tilt 0.37 on a scratch copy of `examples/tilt-demo`, set `alwaysSelect` to `["docs"]` (which has its own Tiltfile resource). Verify with `tilt alpha tiltfile-result -- web` (or a running Tilt and `moon ext tilt -- up web`) that `docs` is enabled with `web`, that no args still enables everything, and what an unknown resource name in `alwaysSelect` does
- [ ] 2.2 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, the workspace tests (after building the release `.wasm` files), and `openspec validate --all --strict`, and verify they pass

## Workflow follow-up

- Archive the change (`/opsx:archive`), updating the `tilt-project-selection` spec.
- Commit as `feat(tilt): add alwaysSelect for infrastructure selected with every project`, closing #48.
