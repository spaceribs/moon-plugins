# Glossary

The Tilt plugins sit between two tools that each have their own vocabulary, and add some of their own. Several words (resource, sync, dependency, service, extension, args, tag) mean different things depending on whose word it is. This glossary splits the terms into three **contexts**, says which one owns each term, and maps the terms across them.

- **[moon](#moon)**: moon's and proto's terms. The plugins read them and don't redefine them.
- **[Tilt](#tilt)**: Tilt's terms. The plugins write Tiltfiles in them and don't redefine them.
- **[The extension](#the-extension)**: terms this repo invented. They exist only here, so they're defined here.

When writing docs, specs, commits, or PRs, use a term in the meaning its context gives it. Qualify a [homonym](#homonyms) when the sentence alone doesn't say which context it's in. A new extension term goes into this file in the same change that introduces it.

## Context map

```
  moon                           the extension                         Tilt
  (upstream: read only)          (translates between them)             (downstream: written to)

  moon query projects/tasks ──▶  exposed tasks, services,      ──▶    Tiltfile: local_resource,
  moon.yml project.tilt.*        build steps, declared                 k8s_resource, dc_resource,
  workspace sync (moon sync)     workloads, images, buttons,           custom_build, cmd_button,
                                 selection                             config.parse() args

  moon ext tilt -- up/focus  ──▶ selection (project IDs)       ──▶    tilt up / tilt args
  moon ext tilt -- exec      ◀── where a workload runs         ◀──    Tilt API: KubernetesDiscovery,
                                                                       DockerComposeService, Cluster
```

The extension conforms to both sides. It never changes what a moon or Tilt term means. Instead it translates: moon's model goes in through `moon query` and project metadata, and Tilt's model comes out as generated Tiltfiles and goes back in through Tilt's API. Code follows the same split:

| Context | Code |
| --- | --- |
| moon, as read by the extension | `extensions/tilt/src/query.rs` (views of `moon query` output), `config.rs` (workspace and project settings), `toolchain.rs` (proto's install layout) |
| Translation | `tiltfile.rs` (tasks to resources), `watch.rs` (inputs to watch rules), `workloads.rs`, `images.rs`, `buttons.rs`, `exec.rs` |
| Tilt, as written by the extension | `template.rs` and `templates/` (Starlark output) |

### Translation table

| moon | The extension | Tilt |
| --- | --- | --- |
| task, target `web:dev` | exposed task | resource named `web:dev` |
| persistent task (`preset: server`) | service | `local_resource(serve_cmd=...)` |
| other task | build step | `local_resource(cmd=..., allow_parallel=True)` |
| task `deps` | task dependency | `resource_deps` |
| project `dependsOn` (in `dependencyScopes`) | runtime dependency | `resource_deps` between services and workloads |
| (none) | runtime-only dependency (`project.tilt.dependsOn`) | `resource_deps` |
| task `inputs`, `outputs`, `.gitignore` | watch rules | resource `deps` and `ignore`, `watch_settings()` |
| project ID | project label, selection key | resource `labels`, Tiltfile args |
| task tags `tilt`, `tilt-exec`, `tilt-confirm` | exposure, exec button, confirmation | `local_resource`, `cmd_button`, `requires_confirmation` |
| project with `docker.file` + committed `Dockerfile` | image (`project.tilt.image`) | `custom_build` |
| (none: deployed by your Tiltfile) | declared workload (`project.tilt.workloads`) | Kubernetes or Compose resource, decorated with `k8s_resource()`/`dc_resource()` |
| affected projects | `--edit-affected` | `--to-edit` Tiltfile arg |

## moon

Terms owned by [moon](https://moonrepo.dev/docs) and [proto](https://moonrepo.dev/proto). Links go to moon's docs.

| Term | Meaning |
| --- | --- |
| **workspace** | The repository moon manages, configured in `.moon/`. Its root is where Tilt runs too. |
| **project** | A directory with a `moon.yml`, identified by its **project ID**. The unit the extension selects by. |
| **task** | A command a project defines in `moon.yml`. Identified by a **target**, `<project>:<task>`. |
| **persistent task** | A long-running task (`persistent: true`, set by `preset: server`). moon never caches it. The extension calls it a [service](#the-extension). |
| **task deps** | Other tasks a task runs after (`deps`). They can carry their own `args` and `env`. Not Tilt's `deps`, which are files to watch. |
| **inputs / outputs** | Files moon hashes to decide whether a task is affected, and files it caches afterwards. |
| **affected** | A task or project whose inputs changed (`moon query projects --affected`). |
| **project graph** | Projects and their `dependsOn` edges, each with a **dependency scope**: `production`, `development`, `peer`, or `build`. |
| **toolchain-inferred dependency** | A `dependsOn` edge moon adds from a toolchain's manifest (e.g. `package.json` `dependencies`) rather than `moon.yml`. |
| **tag** | A label on a project or task in `moon.yml` (`tags: ["tilt"]`). Not a Docker image tag. |
| **project metadata** | Free-form settings under `project:` in `moon.yml`. The extension's per-project settings live under `project.tilt`. |
| **workspace sync** | The step that keeps generated files up to date: `moon sync`, and automatically before `moon run`/`moon ci`. The extension regenerates `.moon/tilt/` here. Skipped with `MOON_SKIP_SYNC_WORKSPACE`. Not a Live Update sync. |
| **`moon run` / `moon exec`** | Run a target and its task deps. `moon exec` is the lower-level form, which also runs `runInCI: false` tasks when `CI` is set. Each exposed task's resource runs `moon exec <target> --upstream none`, so Tilt, not moon, orders the deps. |
| **`moon ci`** | moon's CI command. Not `tilt ci` or `moon ext tilt -- ci`. |
| **`moon docker file` / `scaffold` / `setup`** | Write a project's `Dockerfile`; copy only the project and its dependencies into an image's build stage (writing `dockerManifest.json` at its root); install their toolchains and dependencies there. |
| **plugin** | A WASM module moon or proto loads, referenced by a **locator** (`github://`, `file://`, `https://`). |
| **extension** | A moon plugin run with `moon ext <id>`, configured in `.moon/extensions.yml`. This repo's Tilt extension is one. Not a [Tilt extension](#tilt). |
| **toolchain** | A moon plugin that installs and pins a tool for tasks, configured in `.moon/toolchains.yml`. `tilt_toolchain` is one. |
| **proto tool** | A proto plugin that installs one tool (`proto install tilt`). `tilt_tool` is one, bundled into the toolchain. |
| **workspace-root dependency files** | The root lockfile, workspace or catalog manifest (`pnpm-workspace.yaml`, `go.work`), and root `package.json`/`Cargo.toml`/`pyproject.toml`. moon hashes them through the toolchain, not as task inputs. |

## Tilt

Terms owned by [Tilt](https://docs.tilt.dev). The extension writes these and doesn't change them.

| Term | Meaning |
| --- | --- |
| **Tiltfile** | Tilt's Starlark config. `include()` loads another Tiltfile; `load()` imports names from a `.star` file. |
| **resource** | Anything Tilt shows and runs in its UI: a local resource, a Kubernetes resource, or a Compose service. Names are unique per Tilt instance. |
| **local resource** | A `local_resource()` on the host: `cmd` runs to completion, `serve_cmd` keeps running. |
| **`resource_deps`** | Resources a resource starts after. Tilt ignores names that don't exist. |
| **`deps` / `ignore`** | Files a resource watches, and patterns excluded from them. Not moon's task `deps`. |
| **labels** | Groups resources in Tilt's UI. Not Kubernetes labels. |
| **Kubernetes resource / Compose service** | A resource Tilt deploys from `k8s_yaml()`/`helm()`/`helm_resource` or `docker_compose()`. `k8s_resource()` and `dc_resource()` configure one after it exists. |
| **`custom_build` / `docker_build`** | An image Tilt builds and swaps into the deploys that reference its name. |
| **Live Update** | Copying changed files into a running container instead of rebuilding its image: `sync` steps, `run` steps, `fall_back_on` paths that force a rebuild, and restarts (`restart_process`, `restart_container`). A **Live Update sync** is one of these copies. Not a workspace sync. |
| **Tiltfile args** | Arguments after `--` in `tilt up`/`tilt args`, parsed with `config.parse()`. Remembered in `tilt_config.json`. Not a task's args. |
| **enabled / disabled** | Whether Tilt runs a resource. Disabling keeps it defined, so re-enabling doesn't reload anything. |
| **`tilt up` / `tilt ci` / `tilt down` / `tilt args` / `tilt trigger`** | Start Tilt; run it until every resource is ready and exit; delete what it deployed; change the running instance's Tiltfile args; re-run one resource. |
| **Tilt extension** | A Tiltfile library loaded with `load("ext://<name>", ...)` from [tilt-extensions](https://github.com/tilt-dev/tilt-extensions), such as `helm_resource`, `uibutton`, or `restart_process`. Not this repo's moon extension. |
| **button** | A `cmd_button` in Tilt's UI, on a resource or the navigation bar, from the `uibutton` Tilt extension. |
| **Tilt API objects** | What a running Tilt reports (`tilt get`): `KubernetesDiscovery` (pods), `DockerComposeService` (containers), `Cluster` (kubeconfig and context). |

## The extension

Terms this repo introduced. Where a term reuses a moon or Tilt word, it's narrower than that word, and the entry says how.

### Exposing tasks

| Term | Meaning |
| --- | --- |
| **expose** | Turn a moon task into a Tilt local resource. A task is exposed when it's tagged `tilt`, it's a persistent task in a project tagged `tilt` (or any persistent task, with `includePersistent`), it's an upstream service, or an exposed task depends on it. Use "expose" for tasks; [select](#selecting-what-runs) is for projects. |
| **excluded task** | A tagged task that's never exposed: interactive, runs Tilt itself, or depends on one that does. The generated file lists the last kind. |
| **service** | An exposed persistent task, rendered as `serve_cmd`. Not a Kubernetes Service or a Compose service. |
| **build step** | An exposed non-persistent task, rendered as `cmd`. |
| **upstream service** | A service in a project the exposed services' projects depend on at runtime. Exposed even when untagged (`includeUpstreamServices`). |
| **library** | A project with no services. Service dependencies walk through it to the projects it depends on. |
| **runtime dependency** | A project dependency that links services: moon's `dependsOn` in `dependencyScopes`, plus runtime-only dependencies. |
| **runtime-only dependency** | `project.tilt.dependsOn`: links services and selection like a moon dependency, without affecting images or moon's graph. |
| **`(moon)` resource** | The local resource that regenerates `.moon/tilt/` when moon config changes while Tilt runs (`watchConfig`). |

### Selecting what runs

| Term | Meaning |
| --- | --- |
| **select** | Enable a project's resources in the one Tilt instance, by project ID, and disable everything else. `up` adds to the selection, `focus` replaces it. |
| **selection** | The resources a project ID enables: its own resources, its declared workloads, its `project.tilt.resources`, and the services, workloads, and `project.tilt.resources` of its runtime dependencies. |
| **`project.tilt.resources`** | Names of resources a project's own Tiltfile defines, so its ID selects them. Not the `resources` template variable, which holds only exposed tasks. |
| **reapply marker** | The throwaway `(reapply)` Tiltfile arg `up`/`focus` pass first, so Tilt re-applies an unchanged selection. |
| **edit** | Build a project's image from source rather than use the published image: `--edit api,web`, `--edit-affected`, rendered as the `--to-edit` Tiltfile arg. |

### Workloads and images

| Term | Meaning |
| --- | --- |
| **declared workload** | A Kubernetes resource or Compose service a project's own Tiltfile deploys, named in `project.tilt.workloads`. The extension references it and never generates it. Not a Kubernetes workload kind (Deployment, StatefulSet). |
| **containerized project** | A project with at least one declared workload. Its services run as those workloads, not on the host. |
| **decoration** | The `k8s_resource()`/`dc_resource()` call the extension adds for a declared workload: its `resource_deps` and project label. Tilt merges it with your own call. |
| **project label** | The project ID as a Tilt label (`workloadLabels: "project"`). `moon_labels()` in `lib.star` returns it for resources your own Tiltfile defines. |
| **image** | A project's container image, built from its committed `Dockerfile` when it sets `project.tilt.image`. Your own deploy uses it by that name. |
| **published image** | What an image name resolves to in your registry: used for projects not being edited. |

### Exec buttons

| Term | Meaning |
| --- | --- |
| **exec button** | A Tilt button for a task tagged `tilt-exec` (`execTag`), which runs `moon ext tilt -- exec`. Settings in `project.tilt.buttons`. |
| **confirm** | A task tagged `tilt-confirm` (`confirmTag`), or depending on one: its button asks first, and `exec` needs `--yes`. |
| **exec** | `moon ext tilt -- exec <target>`: `moon run` the task where its project runs, inside its first declared workload (`kubectl exec`/`docker exec`), or on the host. |
| **button args** | `project.tilt.buttons.<task>.args`, the label of a button's text input. Its value reaches `exec` as `MOON_TILT_ARGS` and goes to the task after `--`, after any button argv. |
| **button argv** | `project.tilt.buttons.<task>.argv`, fixed arguments a button always passes to its task after `--`. Only part of Tilt's `cmd_button` `argv`, which is the button's whole command. |
| **button env** | `project.tilt.buttons.<task>.env`, environment variables set only for a button's run, for its task and the task dependencies moon runs with it. Passed to `exec` as `--env KEY=VALUE`. |

### Generated files

| Term | Meaning |
| --- | --- |
| **generation** | Rendering `.moon/tilt/`, on workspace sync, `moon ext tilt -- generate`, or the `(moon)` resource. "Fails generation" means the config is rejected before Tilt sees it. |
| **output directory** | `outputDir`, `.moon/tilt/` by default. Ignores itself, never committed. |
| **main Tiltfile** | Your workspace-root `Tiltfile` (`tiltfile`). Created once if missing, then yours. It includes the generated entry Tiltfile. |
| **entry Tiltfile** | `.moon/tilt/Tiltfile`, which includes project Tiltfiles and then each section file. |
| **project Tiltfile** | A project's own `<project>/Tiltfile`, included by the entry Tiltfile (`includeProjectTiltfiles`). |
| **section** | One generated file (`Tiltfile`, `lib`, `args`, `resources`, `image`, `workloads`, `buttons`, `selection`), rendered from its **section template** (`moon/sections/<section>.tera`). A `<section>.tera` in `templateDir` overrides one. |
| **load check** | `moon ext tilt -- generate --load`: load the main Tiltfile as Tilt does (`tilt alpha tiltfile-result`), without a cluster, and fail with Tilt's error if it doesn't load. Catches what generation can't, such as a declared workload no Tiltfile deploys. |
| **generated header** | The comment each generated file starts with. Generation only rewrites or deletes files that start with it. |

## Homonyms

Words used in more than one context. Say which one you mean.

| Word | moon | Tilt | The extension |
| --- | --- | --- | --- |
| **resource** | (not used) | anything in Tilt's UI | `project.tilt.resources`: names a project's ID selects; `resources` template variable: exposed tasks only |
| **sync** | workspace sync (`moon sync`) | Live Update sync into a container | `sync` setting: regenerate on workspace sync |
| **deps / dependency** | task `deps`; project `dependsOn` | `resource_deps` (order); `deps` (files to watch) | runtime dependency; runtime-only dependency (`project.tilt.dependsOn`) |
| **service** | (not used) | Compose service; Kubernetes Service | an exposed persistent task |
| **workload** | (not used) | (not used; Kubernetes uses it for Deployments etc.) | declared workload |
| **select** | (not used) | (not used) | projects only. Tasks are exposed, not selected. |
| **extension** | moon extension (`moon ext`) | Tilt extension (`ext://`) | this repo's moon extension |
| **tag** | project or task tag | (Docker: image tag) | `tag`, `execTag`, `confirmTag` settings name moon tags |
| **label** | (not used) | resource labels in the UI | project label, `workloadLabels`, `moon_labels()` |
| **args** | task args (after `--`, or on a dependency) | Tiltfile args; `cmd_button` `argv` (the whole command) | button args, `MOON_TILT_ARGS`; button argv |
| **ci** | `moon ci`; the `CI` variable | `tilt ci` | `moon ext tilt -- ci`, which runs `tilt ci` |
| **image** | `docker.file.image`: the base image | an image Tilt builds or deploys | `project.tilt.image`: the image name to build under |
| **trigger** | (not used) | `tilt trigger`, trigger modes | "rebuild triggers": files that rebuild an image |
| **toolchain** | toolchain plugin; toolchain-inferred dependencies | (not used) | the `tilt` toolchain, whose binary `tiltBin` runs |
