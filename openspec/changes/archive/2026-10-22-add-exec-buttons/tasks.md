# Tasks

## 1. Settings and validation

- [x] 1.1 Add workspace settings `execTag` (default `tilt-exec`) and `confirmTag` (default `tilt-confirm`) to `config.rs`, with the extension's config schema and README workspace-settings table updated; verify with a unit test that the defaults deserialize and that both can be overridden
- [x] 1.2 Parse `project.tilt.buttons` (through `tilt_metadata()`) with serde: an object of `{ text?, icon?, args? }` with `deny_unknown_fields` and non-empty strings. Add a validation pass, run in `render_generated` before planning, that reports every malformed entry and every entry naming a task the project doesn't have or that lacks `execTag`, each naming the project and the setting. Verify with unit tests for each error and a WASM integration test that `moon ext tilt -- generate` fails with the message

## 2. Generated buttons

- [x] 2.1 Plan buttons from `moon query tasks`: one per task tagged `execTag`, with target, project, label (`text`, else `description`, else task ID), icon, confirmation (`confirmTag`), args label, the `exec` command (`--yes` when confirmed), and the resource it attaches to (first declared workload, else first planned local resource, else none for the navigation bar). Verify with unit tests for each attachment case, the label fallbacks, and that the tag doesn't create a local resource
- [x] 2.2 Add the `buttons` section template (`templates/sections/buttons.tera`), its `buttons` template variable, and its include in the generated `Tiltfile`. It loads `ext://uibutton` and renders `cmd_button` with `requires_confirmation`, `icon_name`, `location.NAV` when there's no resource, and a `text_input("MOON_TILT_ARGS", …)` when `args` is set; it renders nothing, and loads nothing, without buttons. Verify with render unit tests, a `templateDir` override test, and that the golden single-file comparison still passes
- [x] 2.3 Document buttons in the extension README (a section on exec buttons, plus `project.tilt.buttons` in project settings and `buttons` in the template variables and sections tables), including that buttons are for one-off, non-interactive tasks; verify the README example matches a unit-test fixture

## 3. The exec command

- [x] 3.1 With real Tilt 0.37 on a scratch copy of `examples/tilt-demo` (one Kubernetes and one Compose workload), record the JSON of `tilt get kubernetesdiscovery <workload>`, `tilt get dockercomposeservice <workload>`, and the cluster object Tilt uses for its kube context. Save trimmed copies as test fixtures, and verify the fields `exec` needs (running pod name and namespace, Compose project name and config paths, kube context) are present
- [x] 3.2 Add `moon ext tilt -- exec <project>:<task> [--yes] [-- <args>]`. It resolves the task and its project, refuses `confirmTag` tasks without `--yes`, and takes args after `--`, or else from `MOON_TILT_ARGS` (via `get_host_env_var`), split with shell-word rules. For a host project it streams `moon run <target> -- <args>` from the workspace root. Verify with unit tests for target parsing, the confirmation error, and arg splitting (quotes, escapes, empty), and a WASM integration test for the host path with a fake moon
- [x] 3.3 Add the Kubernetes path to `exec`: find a running pod from the workload's `KubernetesDiscovery` and stream `kubectl --context <ctx> exec -n <ns> <pod> -- moon run <target> -- <args>`. Check that `kubectl` exists first, and fail naming the workload when Tilt isn't running or no pod is running. Verify with unit tests on the 3.1 fixtures for command building and each error
- [x] 3.4 Add the Compose path to `exec`: from the workload's `DockerComposeService` status (running container name), stream `docker exec <container> moon run <target> -- <args>`. Check that `docker` exists first, and fail naming the workload when it isn't running. Verify with unit tests on the 3.1 fixtures
- [x] 3.5 Document `exec` in the README's commands section, including `--yes`, `MOON_TILT_ARGS`, and the requirements per target (`kubectl` or `docker`); verify each documented invocation runs as written against a real Tilt (the extension's CLI has no `--help`)

## 4. Verification

- [x] 4.1 With real moon and Tilt on a scratch copy of `examples/tilt-demo`, tag a task `tilt-exec` (and one also `tilt-confirm`, with an `args` setting) in a host project and in a Kubernetes-containerized project. Verify the Tiltfile loads (`tilt alpha tiltfile-result`), the buttons show on the expected resources, clicking them runs the tasks (confirming first, passing the input's args), and `moon ext tilt -- exec` without `--yes` refuses the confirmed task
- [x] 4.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), creating the `tilt-exec-buttons` spec.
- Commit as `feat(tilt): generate exec buttons from moon tasks tagged tilt-exec`, push `feat/tilt-exec-buttons`, and open a PR that closes #15.
