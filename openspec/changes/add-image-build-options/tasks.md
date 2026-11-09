# Tasks

## 1. Build options

- [ ] 1.1 Parse `project.tilt.imageBuild` (serde, `deny_unknown_fields`: `ssh` and `secrets` lists, `args` object, `target` string) and add `image_build_settings_error`, called from `render_generated` with the other validation passes, reporting every malformed option and options on a project without `project.tilt.image`, each naming the project and option. Verify with unit tests for an unknown key, wrong types, empty values, an invalid build-arg name, and the missing image
- [ ] 1.2 Add the options to the image's `command` in `plan_images`, in the fixed order with shell quoting. Verify with unit tests that a project without the setting keeps today's command, the issue's SSH example, every option together (the spec's scenario), a value needing quotes including a `'`, and that build args are sorted
- [ ] 1.3 Document `imageBuild` in the extension README (the images section with the SSH example, the BuildKit requirement, and not putting credentials in `args`, plus the project-settings table) and add **image build options** to `GLOSSARY.md`'s workloads-and-images table. Verify the README example matches a unit-test fixture

## 2. Verification

- [ ] 2.1 With real Docker (BuildKit) and the generated command, build a scratch project whose Dockerfile has a `RUN --mount=type=ssh` step checking the agent socket, a `RUN --mount=type=secret` step reading the secret, an `ARG` printed by a step, and two stages. Verify the build succeeds with an `ssh-agent` running and `imageBuild` set, and fails at the SSH step without it. If Tilt can build here, also verify through `tilt ci` or `tilt up`
- [ ] 2.2 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, the workspace tests (after building the release `.wasm` files), and `openspec validate --all --strict`, and verify they pass

## Workflow follow-up

- Archive the change (`/opsx:archive`), updating the `tilt-container-images` spec.
- Commit as `feat(tilt): add imageBuild options for image builds`, closing #49.
