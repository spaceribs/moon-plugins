# Design

## Context

- **Tasks:** `moon query tasks` gives each task's `tags` and `description`. moon 2.6 tasks have no free-form metadata, so a task can only carry the opt-in and the confirmation as tags. Display settings go in project metadata, under `project.tilt`, read through `tilt_metadata()` like the other project settings.
- **Workloads:** a project's workloads come from `declared_workloads()` (`project.tilt.workloads`, Kubernetes then Compose). Their names are Tilt resource names. Those aren't necessarily Kubernetes object names: `helm_resource` names the release, and `k8s_resource` can rename a workload.
- **Running commands:** the extension already queries the running Tilt with `tilt get … -o json` (`running_args`). It runs commands with live output through the `exec` host call (`run_tilt`). Host-function errors can't be caught, so a binary that may be missing (`kubectl`, `docker`) must be checked before it's run, as `query_moon` does for moon.
- **Rendering:** each generated file comes from a section template with shared variables. A section that needs a Tilt extension loads it itself, as `image.tera` does with `ext://restart_process`.

## Goals / Non-Goals

**Goals:**
- One place, the moon task, declares that a task is a button and that it's destructive.
- The same command runs a task from a button, a terminal, or an agent, with the same confirmation check.
- Generated Tiltfiles stay platform-neutral. A Windows host gets working buttons for host and workload projects alike.

**Non-Goals:**
- Running buttons when the extension isn't installed. The button calls `moon ext tilt`, which a workspace using the generated Tiltfile already has.
- Choosing a container in a multi-container pod. `kubectl exec` uses the pod's default container (`kubectl.kubernetes.io/default-container`, else the first).

## Decisions

**Buttons call `moon ext tilt -- exec`, not `kubectl exec` directly.** The generated button runs `moon ext tilt -- exec <target> [--yes] [-- <args>]`. The command finds where to run at click time:
- For a Kubernetes workload, it reads the workload's `KubernetesDiscovery` from the running Tilt (`tilt get kubernetesdiscovery <workload> -o json`, named after the Tilt resource even when `k8s_resource` renames it) and runs `kubectl exec` in a running pod, in its namespace. It passes the kubeconfig and context from Tilt's `Cluster` object, so it uses the cluster Tilt deployed to whatever `KUBECONFIG` the caller has.
- For a Compose workload, it reads the `DockerComposeService` (`tilt get dockercomposeservice <workload> -o json`), whose status names the running container, and runs `docker exec <container>`. That needs only `docker`, not the project's Compose files. Recorded against Tilt 0.37.8, it replaces the `docker compose exec -p … -f …` first planned.
- For a host project, it runs `moon run` directly.

Alternatives considered:
- Inlining `kubectl exec deploy/<workload>` as the issue sketches. It's simpler, but it assumes the Tilt resource name is a Deployment's name, which breaks for `helm_resource`, renamed resources, StatefulSets, and namespaces. It needs a POSIX shell to splice in the text input on Windows. It also leaves agents without a single entry point that enforces `tilt-confirm`.
- Using Tilt's own pod reference from `UIResource` status. `KubernetesDiscovery` is the documented object that tracks a resource's pods, with their namespaces and phases, so `exec` reads that instead.

**Text input reaches `exec` through an environment variable.** Tilt passes `text_input` values only as environment variables, never in `argv`. When `args` is set, the button sets `env` and declares a text input named `MOON_TILT_ARGS`. `exec` reads it with the PDK's `get_host_env_var` when no args follow `--`, and splits it with shell-word rules (quotes and backslash escapes, no expansion) on every platform. So no shell runs, and Windows and POSIX hosts behave the same. The alternative, a per-platform `sh -c` / `cmd /C` argv chosen in Starlark with `os.name`, would make quoting differ by platform.

**Confirmation is checked in two places.** The button sets `requires_confirmation = True` and passes `--yes`, since Tilt has already asked. `exec` refuses a `confirmTag` task without `--yes`. That makes the check hold for anything that doesn't go through Tilt's UI, which is the issue's point about agents.

**Buttons attach to the first workload, then the first resource, then the navigation bar.** Tilt requires a `cmd_button`'s `resource` to exist. Declared workloads always exist (misnamed ones already fail loading), and planned local resources are generated. A project with neither uses `location.NAV`, so a button never points at a missing resource. Buttons on a disabled resource still render, so selection needs no change.

**A new `buttons` section.** It's rendered to its own file and included like the others, and it loads `ext://uibutton` only when there's a button. Its template variable is `buttons`, a list of the fields the spec names. A `buttons` section keeps the override surface the same as `workloads` and `image`.

**Validation follows the existing pattern.** `project.tilt.buttons` is parsed with serde (`deny_unknown_fields`), and its entries are checked against the project's tasks and tags in a validation pass before planning, like `runtime_dependency_errors` and `live_update_settings_error`. Generation fails with every problem listed, each naming the project.

**`execTag` and `confirmTag` are workspace settings.** They parallel `tag`, which already renames the resource opt-in. The defaults match the issue.

## Risks / Trade-offs

- [Each click starts moon and the WASM plugin, then a `tilt get` and `kubectl exec`] → A second or two of overhead on a manual action. Acceptable, and it's why resolution happens at click time rather than baked into the Tiltfile.
- [`KubernetesDiscovery` or `DockerComposeService` field names differ from what's documented] → Verify against real Tilt 0.37 before writing the parser (task 3.1). The config notes that documented behavior has differed in practice.
- [`kubectl` or `docker` missing on the host] → Checked before running, with an error naming the binary.
- [A `tilt-exec` task that's persistent, or that waits on stdin] → It runs until it's cancelled, or fails without a TTY. The README says buttons are for one-off tasks. Generation doesn't reject persistent tasks, since a long-running task with output in Tilt's log can be legitimate.
- [`ext://uibutton` is fetched from GitHub when the Tiltfile loads] → Same exposure as `ext://restart_process`. It's only loaded when a button exists.

## Migration Plan

Opt-in: nothing changes until a task carries `execTag`. A workspace with hand-written buttons can tag the tasks and delete its own `cmd_button` calls. If both exist with the same name, Tilt reports the duplicate. Release as `feat(tilt)` (minor).
