# Tilt extension

A moon extension that runs your moon workspace in [Tilt](https://tilt.dev): one Tilt instance for the whole monorepo, generated from moon's task and project graphs.

- **moon tasks become Tilt resources.** Tilt's UI, logs, and orchestration work on top of moon's caching, toolchains, and environment.
- **Dependencies come from moon.** Task `deps` and project `dependsOn` decide what starts after what. Services pull in the services they depend on.
- **Only affected things re-run.** Tilt watches exactly the files moon hashes as inputs, honors `.gitignore`, and leaves source edits to your dev server's own live reload.
- **Select by project:** `moon ext tilt -- up web` runs `web` and everything it needs. Add or swap projects in the running instance.
- **Kept in sync.** The generated Tiltfile updates on `moon sync`, before `moon run`, and while Tilt runs.
- **Containers (opt-in).** Projects can be built from moon's generated Dockerfiles, with Live Update and only affected images built from source. The Kubernetes or Compose resources your own Tiltfiles deploy join moon's project graph.

Pair it with the [Tilt toolchain](../../toolchains/tilt) to install and pin the Tilt binary.

## Contents

- [Requirements](#requirements)
- [Quick start](#quick-start)
- [Usage](#usage)
- [How it works](#how-it-works)
  - [How tasks map to Tilt](#how-tasks-map-to-tilt)
  - [Service dependencies from the project graph](#service-dependencies-from-the-project-graph)
  - [What triggers a re-run or restart](#what-triggers-a-re-run-or-restart)
  - [One Tilt instance, selected by project](#one-tilt-instance-selected-by-project)
  - [Workloads your Tiltfiles deploy](#workloads-your-tiltfiles-deploy)
  - [The generated Tiltfile](#the-generated-tiltfile)
- [Containers (opt-in)](#containers-opt-in)
- [Templates](#templates)
- [Configuration](#configuration)

## Requirements

- **moon** 2.6 or newer. Developed and tested with 2.6.2.
- **Tilt.** Tested with 0.37.8. Install it yourself, or with the [Tilt toolchain](../../toolchains/tilt) / [proto plugin](../../tools/tilt). With the toolchain configured, the extension runs the binary it installed for the pinned version (see `tiltBin`), so Tilt doesn't need to be on your `PATH`.
- **Linux or macOS.** Windows isn't supported yet: container image builds run through `sh`.
- For containers only: **Docker**, plus a local Kubernetes cluster such as [kind](https://kind.sigs.k8s.io) created with [ctlptl](https://github.com/tilt-dev/ctlptl) (`ctlptl create cluster kind --registry=ctlptl-registry`).

## Quick start

1. Add the extension:

   ```yaml
   # .moon/extensions.yml
   tilt:
     plugin: "github://spaceribs/moon-plugins/tilt_extension"
   ```

2. Mark what Tilt should run. Tag a project to expose all of its persistent (`preset: server`) tasks, or tag individual tasks:

   ```yaml
   # apps/web/moon.yml
   tags: ["tilt"]
   tasks:
     dev:
       command: "vite dev"
       preset: "server"
       deps: ["codegen"]
   ```

3. Start Tilt:

   ```sh
   moon ext tilt -- up          # everything
   moon ext tilt -- up web      # just web, and everything it needs
   ```

   This generates `Tiltfile.moon`, creates a `Tiltfile` that includes it (if you don't have one), and runs `tilt up`. Open the Tilt UI at <http://localhost:10350>. Resources are grouped by project.

## Usage

```sh
moon sync                              # (re)generate Tiltfile.moon (+ Tiltfile if missing)
moon ext tilt -- generate [--check]    # same, on demand (--check fails if stale, for CI)
moon ext tilt -- up                    # start Tilt with everything
moon ext tilt -- up web api            # start Tilt with these projects, or add them to the running Tilt
moon ext tilt -- focus api             # run only these projects in the running Tilt (starts it if needed)
moon ext tilt -- ci [projects...]      # `tilt ci` for these projects (or everything)
moon ext tilt -- down                  # `tilt down`: tear down resources Tilt created
```

Extra Tilt flags go after another `--`, e.g. `moon ext tilt -- up web -- --legacy`. With containers, `up`, `focus`, and `ci` also take `--edit`/`--edit-affected` ([see below](#building-only-some-images-from-source)).

## How it works

### How tasks map to Tilt

| moon | Tilt |
| --- | --- |
| target `web:dev` | resource name `web:dev` |
| persistent task (`preset: server`) | `serve_cmd` |
| other tasks | `cmd` with `allow_parallel = True` |
| task `deps` | `resource_deps`. Dependencies are pulled in transitively, so Tilt orchestrates the whole subgraph. A dependency declared with `args`/`env` gets its own resource (e.g. `web:codegen[--prod MODE=x]`), so it runs exactly as moon would run it. |
| project `dependsOn` (explicit or toolchain-inferred) | `resource_deps` between services. See below. |
| `inputs`, `outputs`, `.gitignore` | `deps` and `ignore`, matching exactly what moon hashes. See below. |
| project ID | `labels` (groups resources in the UI) |

Every resource runs `moon exec <target> --upstream none --on-failure bail --ignore-ci-checks`, which is what `moon run` does, except that it also runs in CI. `moon run` skips tasks with `runInCI: false` (the default for persistent tasks) when `CI` is set, which would stop `moon ext tilt -- ci` from starting any server there. moon still handles caching, toolchains, and environment, while Tilt owns ordering. Interactive tasks, and tasks that run Tilt itself, are never exposed, even when tagged. Neither are tasks that depend on one, directly or through other such tasks: a `tilt-open` task that runs after `tilt-up` is meant to follow Tilt from moon, and as a resource it would start before Tilt is up (Tilt ignores `resource_deps` on resources that don't exist). The generated file lists the ones it left out, with the dependency that caused it:

```python
# Not exposed: these tasks depend on a task that runs Tilt (directly, or
# through another task listed here), so in Tilt they would start before it.
#   web:tilt-open (depends on web:tilt-up)
```

### Service dependencies from the project graph

moon's project graph decides which services wait on which. When a selected service's project depends on another project, the service gets `resource_deps` on that project's services:

```
web (dev)  ──dependsOn──▶  api-client (library)  ──dependsOn──▶  api (serve)  ──dependsOn──▶  db (serve)

web:dev  waits on  api:serve  waits on  db:serve
```

- **Libraries are walked through.** Projects without services (like `api-client`) are skipped, and the walk continues to their own dependencies until it reaches projects that have services.
- **Upstream services are always included.** `db:serve` isn't tagged, but `api:serve` depends on it, so it's added to Tilt. Set `includeUpstreamServices: false` to only link services that are already selected.
- **Only runtime scopes count.** By default only `production`, `development`, and `peer` dependencies are followed. `build`-scoped dependencies (tooling, codegen) are not services your app talks to. Change this with `dependencyScopes`.
- **Only services are linked.** Non-persistent resources only follow their task `deps`.

Tilt considers a local `serve_cmd` resource ready as soon as it starts. So for local services "waits on" means "starts after", not "starts after it's accepting connections". For [workloads](#workloads-your-tiltfiles-deploy), give the container a readiness probe in your manifest.

### What triggers a re-run or restart

Tilt notices file changes, moon decides what's affected, and your app's own live reload handles the rest. This follows Tilt's [Live Update](https://docs.tilt.dev/tutorial/5-live-update.html) guidance: frameworks that hot-reload (webpack, vite, nodemon, `node --watch`, ...) should get the change without a restart.

**Build steps** (non-persistent tasks) re-run when a file **moon would hash as an input** changes:

- input globs are watched precisely: `.dockerignore`-style `ignore` patterns re-include only matching files, and `{a,b}` braces are expanded
- gitignored files are ignored, as moon ignores them when hashing (from the workspace and each project's `.gitignore`)
- negated inputs (`!**/*.test.ts`) and the task's own outputs are ignored
- explicit input files are always watched, even if gitignored, as moon hashes them
- inputs under `.moon/` and workspace-wide globs (`/**/*`) are not watched

Even when a build step re-runs, the task is a moon cache hit (no work) if moon's hash didn't change.

**Services** (persistent tasks) follow `serviceRestart`:

| Value | Restarts when | Use for |
| --- | --- | --- |
| `upstreamOutputs` (default) | an upstream task dependency's **outputs** change. moon only rewrites outputs when it actually rebuilds; cache hits leave them untouched. | Servers with live reload (source edits are theirs), and compiled servers fed by a build task with `outputs`. |
| `inputs` | the service task's own inputs change (same rules as build steps). | Servers without live reload and without a build task. |
| `never` | never; restart from the Tilt UI. | Anything else. |

Override it per project in `moon.yml` through moon's custom project metadata:

```yaml
project:
  tilt:
    serviceRestart: "inputs"
```

**Your own resources** (in your `Tiltfile` or project Tiltfiles) get the gitignore patterns too, through `watch_settings()`. That means no rebuilds for `node_modules`/`dist` churn, and no inotify watches spent on them. In a test with 6,000 `node_modules` directories, Tilt used 3 watches instead of 6,004. Paths that moon resources need despite being gitignored stay watched: explicit input files, outputs services restart on, moon config, and Tilt's own Tiltfiles. Disable with `globalGitignore: false`.

### One Tilt instance, selected by project

There is always a single Tilt instance for the workspace. You choose what runs in it by **moon project ID**. A project selects its resources, and Tilt brings up their dependencies too. Everything else is disabled, not stopped and restarted.

The generated file accepts project IDs as Tiltfile args, so the same selection works from any entry point:

| From | Example |
| --- | --- |
| moon | `moon ext tilt -- up web` (add), `moon ext tilt -- focus web` (replace) |
| Tilt CLI | `tilt up -- web`, `tilt args -- web docs`, `tilt args --clear` (everything) |
| Tilt UI | edit the Tiltfile args |
| `tilt_config.json` | `{"to-run": ["web"]}` next to the `Tiltfile` remembers a developer's selection |

Raw Tilt resource names work too (`tilt args -- web:dev`). Unknown names are rejected by Tilt, which keeps the current selection.

Resources defined in a project's own `Tiltfile` aren't known to moon. List them in the project's metadata so the project ID selects them as well:

```yaml
# docs/moon.yml
project:
  tilt:
    resources: ["docs-site"]
```

Selecting projects disables everything not selected, including resources from your own `Tiltfile`. Name them alongside projects (`tilt args -- web my-db`), or list them in a project's `project.tilt.resources` so that project brings them up.

`up` and `focus` talk to the instance on `TILT_PORT`/`TILT_HOST` (default `localhost:10350`), like every `tilt` command. If your own Tiltfile already defines Tiltfile args with `config.parse()`, set `selectByProject: false`.

### Workloads your Tiltfiles deploy

Projects that deploy themselves from their own `Tiltfile` (raw YAML, kustomize, Helm, or Docker Compose) can declare those Tilt resources by name, so they join moon's project graph. The extension never generates or reads the deployment. It only references it:

```yaml
# apps/api/moon.yml
project:
  tilt:
    workloads:
      k8s: ["api", "api-worker"]   # k8s_yaml, kustomize, helm(), helm_resource
      compose: []                  # docker_compose services
```

```python
# apps/api/Tiltfile
k8s_yaml("k8s.yaml")
k8s_resource("api", port_forwards=[3001], labels=["team-a"])
```

A project with declared workloads is **containerized**:

- **Its ID selects its workloads.** `moon ext tilt -- up api` enables `api` and `api-worker`, and their dependencies.
- **The workloads join the graph.** Each workload waits on the project's upstream services (pulling them in), and services of projects that depend on `api` wait on every one of its workloads.
- **Its services don't also run on the host.** Persistent tasks, and anything selected through the project's tag or `includePersistent`, aren't local resources, even when a persistent task is tagged itself. A non-persistent task tagged itself (a codegen or schema-export step, say) is still a local resource, with its task dependencies.

The extension decorates each workload with `k8s_resource(...)` or `dc_resource(...)`, adding `resource_deps` and a label with the project ID. Tilt merges these with your own calls, so labels, dependencies, port-forwards, and other settings from your Tiltfile are kept. If your Tiltfiles already label workloads (by role, for example) and the extra project label repeats them in the Tilt UI, set `workloadLabels: "none"` in the workspace settings to add only `resource_deps`.

- **Names must match what your Tiltfile deploys**, including Helm releases from the [`helm_resource`](https://github.com/tilt-dev/tilt-extensions/tree/master/helm_resource) extension, which go under `k8s`. A declared name that nothing deploys fails Tiltfile loading with Tilt's `k8s_resource specified unknown resource "<name>"`.
- Declare each kind under its own key: `k8s_resource` and `dc_resource` only accept their own kind of resource.
- **Load Compose files before the decorations.** For Kubernetes the order doesn't matter, but Tilt rejects `dc_resource` for a service no `docker_compose()` has loaded yet (`no Docker Compose service found with name "<name>"`). Project Tiltfiles are included before the decorations, so call `docker_compose()` there, or before `include("./Tiltfile.moon")` in your main `Tiltfile`.

### The generated Tiltfile

The extension keeps a top-level Tiltfile in sync with your workspace, so `tilt up` from the workspace root brings up everything:

```
Tiltfile            yours. Created once; include()s Tiltfile.moon. Add workspace-wide resources here.
Tiltfile.moon       generated. Includes every project's own Tiltfile, then a resource per moon task.
<project>/Tiltfile  optional per-project resources (k8s_yaml, docker_compose, ...)
```

If you already have a `Tiltfile`, add `include("./Tiltfile.moon")` to it.

`Tiltfile.moon` is regenerated during moon's **workspace sync**: on `moon sync`, and automatically before `moon run`/`moon ci`. Adding a project, a `Tiltfile`, a tag, or a dependency updates it without extra steps.

- The file is only rewritten when its contents change, so a running `tilt up` reloads only when something actually changed.
- Resources launched by Tilt run moon with `MOON_SKIP_SYNC_WORKSPACE=true`, so service restarts don't trigger syncs.
- Sync failures (for example, `moon query` erroring, or a `moonBin` path that doesn't exist) are recorded on the sync operation, not raised, so they never block `moon run`. The one exception is a bare `moonBin` name (the default `moon`) that isn't on `PATH`: the host aborts that call. If moon isn't on your `PATH`, set `moonBin` to its path, or set `sync: false`.
- In CI, moon flags files changed by a sync. Either commit `Tiltfile`/`Tiltfile.moon`, or ignore them and set `sync: false` for CI.
- Project Tiltfiles are evaluated relative to their own directory, as with any Tilt `include()`. Resource names must be unique across the whole workspace.
- On macOS, `cd` into the workspace with its exact directory casing. A mismatch (e.g. `~/projects` when the folder is `~/Projects`) breaks Tilt's ignore patterns ([tilt#5271](https://github.com/tilt-dev/tilt/issues/5271)).

**While Tilt runs**, a `(moon)` resource watches everything the generated file is derived from: `.moon/*` config and `.moon/tasks/`, every project's `moon.yml`, the `.gitignore` files, and a custom template. When one changes, it runs `moon ext tilt -- generate`, and Tilt reloads only if the output changed. It doesn't run at startup, and stays enabled whatever projects are selected. Disable with `watchConfig: false`. New projects matched by workspace globs are picked up the next time anything regenerates the file.

## Containers (opt-in)

Projects can also run as containers, built from moon's [generated Dockerfiles](https://moonrepo.dev/docs/commands/docker/file). The extension builds the image. Your own Kubernetes YAML, Helm chart, or Compose file deploys it, and [declaring that workload](#workloads-your-tiltfiles-deploy) connects it to moon's project graph. Both are opt-in per project, so `docker.file` settings you use for production images are left alone:

| `moon.yml` | Result |
| --- | --- |
| `project.tilt.image: "name"` | image built from moon's Dockerfile, for your own resources to deploy |
| `project.tilt.workloads: {...}` | project containerized: its services run as those workloads |
| `docker.file` settings only | nothing |

```yaml
# apps/api/moon.yml
docker:
  file:
    image: "node:22"    # base image; moon's Dockerfile installs moon with curl and bash
    startTask: "serve"  # what the container runs
project:
  tilt:
    image: "tilt.dev/api"
    workloads:
      k8s: ["api"]
```

```python
# apps/api/Tiltfile
k8s_yaml("k8s.yaml")   # a Deployment whose container uses image "tilt.dev/api"
k8s_resource("api", port_forwards=[3001])
```

Upgrading from `project.tilt.k8s`? See [Migrating from generated manifests](#migrating-from-generated-manifests).

### Images

Each image is a `custom_build`. It runs `moon docker file <project>` (into `.moon/cache/tilt/`), then `docker build` with the **whole workspace as context**, which moon's Dockerfiles need for `moon docker scaffold`. The image is named `project.tilt.image`. What triggers a rebuild comes from moon's project graph, not the build context:

- the source directories of the project and every project it depends on (transitively, all scopes)
- their moon config (`.moon/*`, `moon.yml`), which the Dockerfile is generated from
- root-level files their tasks take as inputs (lockfiles, shared configs)
- minus their task outputs (the image builds inside the container, not from host artifacts) and gitignored files

So editing one service never rebuilds unrelated images, while a change to a shared library rebuilds every image that uses it.

Every build sends the whole workspace to Docker, so keep the context small with a `.dockerignore` at the workspace root:

```
.git
.moon/cache
.moon/docker
node_modules
```

[moon's Docker guide](https://moonrepo.dev/docs/guides/docker) recommends ignoring `.moon/cache` (not portable between machines) and `.git` (moon still works in the image, with its caching disabled, and the image stays smaller). The generated Dockerfiles live in `.moon/cache/tilt/` and still build when it's ignored.

**Base images:** moon's generated Dockerfile installs moon with `curl ... | bash`, so the `docker.file.image` needs both (e.g. `node:22`, not `node:22-alpine`). For Alpine, use a [custom Dockerfile template](https://moonrepo.dev/docs/config/workspace#docker) that installs moon with `npm install -g @moonrepo/cli`, and set `ENV MOON_TOOLCHAIN_FORCE_GLOBALS=true`, as moon's guide shows.

**Plugins inside the image build:** moon runs inside the build (`moon docker scaffold`, `moon docker setup`), so it loads your workspace's plugins there too.

- **Published locators** (`github://`, `https://`) are downloaded inside the build and just work.
- **`file://` locators must point inside the workspace.** Paths outside it don't exist in the container.
- **Vendored plugins (e.g. `.moon/plugins/`) must be added to the scaffold**, because moon only copies scaffolded config into the build stage. Add them to the configs phase in `.moon/workspace.yml`:

  ```yaml
  docker:
    scaffold:
      configsPhaseGlobs: [".moon/plugins/**/*"]
  ```

  (A [custom Dockerfile template](https://moonrepo.dev/docs/config/workspace#docker) with `COPY --from=skeleton /app/.moon/plugins ./.moon/plugins` after its configs `COPY` works too.)

### Deploying your image

Tilt only builds images that a deploy uses, matching on the image name:

- **Use `project.tilt.image` verbatim in your manifest or Compose file**, registry prefix included (`tilt.dev/api`, `localhost:5000/api`). Tilt rewrites the reference to the image it built. If the names don't match, Tilt silently runs the published image instead.
- **Remove your own build of the same image.** If your Tiltfile already calls `docker_build("tilt.dev/api", ...)` or `custom_build(...)` for that name, Tilt refuses to load: `Image for ref "tilt.dev/api" has already been defined`. Delete it when you set `project.tilt.image`.
- **Don't set `command:` if you rely on restarts.** With `serviceRestart: "inputs"`, Live Update restarts a Kubernetes container's process by wrapping the image's entrypoint (the project's `docker.file.startTask`). A `command:` in your manifest replaces that entrypoint, so the process is never restarted after a sync. Use `args:` instead, or let the app reload itself. (Compose services are restarted as a whole container, so a Compose `command:` is simply what re-runs.)
- **Memory:** the container runs `moon run <startTask>`, so moon itself (and the task's dependencies) runs in the pod alongside your app. Size limits for both. A small Node service peaked at about 374 MB in testing, and was OOM-killed at a 256Mi limit.
- **Readiness:** Tilt reports a pod healthy once its containers are ready, so give it a readiness probe (e.g. a TCP check on its port). Without one, Tilt and `tilt ci`, and the services waiting on it, consider it up as soon as the container starts.

`--edit` decides whether the image builds from source or uses the published one ([below](#building-only-some-images-from-source)).

### Live Update

Source changes are synced into running containers instead of rebuilding the image ([Tilt Live Update](https://docs.tilt.dev/live_update_reference.html)). In testing, a source edit was live in about 1 second, compared with a full rebuild and redeploy.

- **Synced:** the project's directory and those of the projects it depends on, to the same path under the image's `WORKDIR` (`/app` in moon's Dockerfiles), e.g. `api` → `/app/api`.
- **Full rebuild instead:** moon config, root-level task inputs (lockfiles), and explicit task input files (manifests like `package.json`), since those change dependencies or the Dockerfile itself.
- **After syncing**, the same `serviceRestart` setting as local services decides:
  - default: the app is expected to reload itself (webpack, vite, nodemon, `node --watch`)
  - `serviceRestart: "inputs"`: the process is restarted with Tilt's [`restart_process`](https://github.com/tilt-dev/tilt-extensions/tree/master/restart_process) extension, re-running the project's `docker.file.startTask`. The extension is fetched from GitHub when the Tiltfile loads. Projects whose [declared workloads](#workloads-your-tiltfiles-deploy) are all Compose services restart the container instead (`restart_container()`), because Tilt doesn't support `restart_process` for Compose. The container re-runs its CMD, which is the same `startTask`.

Disable it per project with `project.tilt.liveUpdate: false`, or for the workspace with `liveUpdate: false`. Edits to a custom moon Dockerfile template (e.g. `.moon/Dockerfile.tera`) aren't watched; trigger a rebuild from the Tilt UI after changing one.

### Building only some images from source

By default every image builds from source. To build only what you're working on, and use the published images (whatever the image name resolves to in your registry) for everything else:

```sh
moon ext tilt -- up --edit api,web          # only api and web build from source
moon ext tilt -- up --edit-affected         # projects your changes affect, plus their dependents
moon ext tilt -- up --edit-affected --base main   # ...compared to main (committed work on a branch)
```

`--edit-affected` uses moon's own change detection (`moon query projects --affected --downstream deep`). With no `--base`, it covers uncommitted changes. When nothing is affected, nothing builds from source. On a running Tilt these flags update the choice in place, and later `up`/`focus` calls keep it. From Tilt directly: `tilt args -- web --to-edit api` (`--to-edit none` for no source builds).

`focus` and `ci` take the same flags, so CI can bring up the stack with only the changed services built: `moon ext tilt -- ci --edit-affected --base origin/main`.

### Migrating from generated manifests

Earlier versions generated a Deployment and Service from `project.tilt.k8s`. That was removed: generated manifests couldn't express what real workloads need (liveness probes, init containers, Secrets, your own label conventions). Generation now fails with an error naming each project that still sets `project.tilt.k8s`. To migrate one:

1. Export what the extension used to generate:

   ```sh
   moon ext tilt -- export-k8s api                              # writes api/k8s.yaml
   moon ext tilt -- export-k8s api --output deploy/k8s/api.yaml # or anywhere in the workspace
   ```

   It writes the Deployment, a Service when you had ports, and the readiness probe, as YAML for you to maintain. It never overwrites an existing file.

2. Replace `project.tilt.k8s` in the project's `moon.yml` with what the command prints. `image` keeps the image building from moon's Dockerfile (the generator implied it), and `workloads` keeps the project containerized:

   ```yaml
   project:
     tilt:
       image: "api"
       workloads: { k8s: ["api"] }
   ```

3. Add the printed lines to the project's `Tiltfile` (create it if needed; the generated file includes it):

   ```python
   k8s_yaml("k8s.yaml")
   k8s_resource("api", port_forwards=[3001])
   ```

Labels and `resource_deps` from the project graph are still added for you. If your Tiltfile already built the same image with `docker_build`/`custom_build`, remove that (see [Deploying your image](#deploying-your-image)).

## Templates

Like moon's `Dockerfile.tera`, the generated file is rendered with [Tera](https://keats.github.io/tera/docs/). The built-in template ([`templates/Tiltfile.moon.tera`](./templates/Tiltfile.moon.tera)) is used unless you configure your own:

```yaml
# .moon/extensions.yml
tilt:
  plugin: "github://spaceribs/moon-plugins/tilt_extension"
  template: ".moon/tilt/Tiltfile.tera"   # workspace-relative; an error if missing
```

You can also try one without configuring it: `moon ext tilt -- generate --template path/to/Tiltfile.tera`. The `--template` flag takes precedence over the setting.

### Extending the built-in template

The built-in template is registered as `moon/Tiltfile.moon.tera`, so a custom template can extend it and override only the blocks it needs. For example, deploy Kubernetes manifests for every project that has services:

```jinja
{% extends "moon/Tiltfile.moon.tera" %}

{% block footer %}
{%- for project in projects %}{% if project.resources %}
k8s_yaml(kustomize({{ project.source ~ "/k8s" | moon_path }}))
{%- endif %}{% endfor %}
{% endblock footer %}
```

Blocks, in order:

- `header`
- `preamble`: defines `_MOON_ROOT`, `_moon_path()`, `_MOON_ENV` and `_MOON_GITIGNORE`, and calls `watch_settings()`
- `args`: parses Tiltfile args into `_moon_cfg`, and defines `_moon_edit()`
- `project_tiltfiles`
- `moon_resource`: the `(moon)` resource
- `resources`
- `images`: container builds
- `workloads`: decorations for [declared workloads](#workloads-your-tiltfiles-deploy)
- `selection`: project selection
- `footer`: empty by default

If you override `preamble`, keep those definitions, since the other blocks and the macro use them. Override `images` to add your own `live_update` steps, such as `run(...)` commands after syncing.

### Writing a template from scratch

Import the macros to reuse the standard `local_resource` rendering. The macro expects `_MOON_ROOT`, `_moon_path()`, `_MOON_ENV` and `_MOON_GITIGNORE` to be defined:

```jinja
{% import "moon/macros.tera" as moon -%}
_MOON_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), {{ root | starlark }}))
def _moon_path(path):
    return os.path.join(_MOON_ROOT, path)
_MOON_ENV = {"MOON_SKIP_SYNC_WORKSPACE": "true"}
_MOON_GITIGNORE = {{ gitignore | moon_path }}

{% for resource in resources | filter(attribute="persistent", value=true) -%}
{{ moon::resource(resource=resource) }}
{% endfor %}
```

### Variables

| Variable | Description |
| --- | --- |
| `version` | Extension version. |
| `root` | Relative path from the generated file's directory to the workspace root. |
| `output` | Workspace-relative path of the generated file. |
| `tag` | The configured `tag`. |
| `project_tiltfiles` | Workspace-relative project Tiltfiles to include. |
| `resources` | Resources: `target`, `project`, `task`, `persistent`, `cmd`, `watch`, `only_inputs`, `ignore`, `respect_gitignore`, `resource_deps`, `labels`, `env`. |
| `projects` | Every project: `id`, `source`, `tags`, `dependencies` (`id`, `scope`), `tiltfile` (if included), `resources` (targets and workload names), `selection` (what its ID selects). |
| `gitignore` | Ignore patterns translated from `.gitignore` files (rendered as `_MOON_GITIGNORE`). |
| `gitignore_exceptions` | `!` patterns re-including gitignored paths that must stay watched. |
| `images` | Images: `project`, `image_ref`, `dockerfile`, `command`, `watch`, `ignore`, `live_update` (`fall_back_on`, `syncs` with `local`/`remote`, `restart_entrypoint`, or none). |
| `workloads` | Declared workloads: `name`, `kind` (`k8s` or `compose`), `project`, `labels` (empty with `workloadLabels: "none"`), `resource_deps`. |
| `excluded` | Tasks left out because they depend on a task that runs Tilt, and would otherwise have been resources: `target`, `dependency`. |
| `restart_process` | Whether any image restarts after syncing (loads Tilt's `restart_process` extension). |
| `config_files` | Files the generated file is derived from (watched by `(moon)`). |
| `regenerate_cmd` | Command `(moon)` runs. |
| `global_gitignore`, `watch_config`, `select_by_project` | The corresponding settings. |

### Filters

| Filter | Example | Output |
| --- | --- | --- |
| `starlark` | `{{ ["a", true] \| starlark }}` | `["a", True]`: any value as a Starlark literal (strings quoted and escaped). |
| `moon_path` | `{{ "web/k8s" \| moon_path }}` | `_moon_path("web/k8s")`; lists become lists, and `!`-prefixed patterns stay negated. |

Tera's own [built-in filters and tests](https://keats.github.io/tera/docs/#built-ins) are available, except ones that need optional dependencies (dates, `slugify`, `filesizeformat`, random). One gotcha: `~` binds tighter than filters, so `"a" ~ list | join(sep=",")` concatenates the list before joining it. `set` the filtered value first.

## Configuration

### Workspace settings

In `.moon/extensions.yml`, next to `plugin`:

| Setting | Default | Description |
| --- | --- | --- |
| `tag` | `"tilt"` | Tag that selects tasks, or projects whose persistent tasks are selected. |
| `includePersistent` | `false` | Expose every persistent task, regardless of tags. |
| `linkProjectDependencies` | `true` | Link services to upstream services through moon's project graph. |
| `includeUpstreamServices` | `true` | Expose upstream services even when they aren't tagged. |
| `dependencyScopes` | `["production", "development", "peer"]` | Project dependency scopes treated as runtime relationships. |
| `serviceRestart` | `"upstreamOutputs"` | When services restart: `upstreamOutputs`, `inputs`, or `never`. |
| `workloadLabels` | `"project"` | Label added to [declared workloads](#workloads-your-tiltfiles-deploy): `project` (the project ID) or `none`. |
| `watchInputs` | `true` | Watch files at all. When `false`, nothing re-runs on file changes. |
| `globalGitignore` | `true` | Apply gitignore patterns to every Tilt resource (`watch_settings`), keeping paths moon needs watched. |
| `includeProjectTiltfiles` | `true` | `include()` each project's own `Tiltfile` from the generated file. |
| `selectByProject` | `true` | Accept moon project IDs as Tiltfile args (`tilt up -- web`), and `--to-edit`. |
| `sync` | `true` | Regenerate during moon's workspace sync (`moon sync`, before `moon run`). |
| `watchConfig` | `true` | Regenerate from a `(moon)` resource when moon config changes while Tilt runs. |
| `output` | `"Tiltfile.moon"` | Generated file (workspace-relative). |
| `tiltfile` | `"Tiltfile"` | Main Tiltfile, created if missing and used by `up`/`ci`/`down`. |
| `template` | none | Workspace-relative Tera template used to render the generated file. |
| `images` | `true` | Build images for projects that opt in (see below). `false` turns containers off entirely. |
| `liveUpdate` | `true` | Sync source changes into running containers instead of rebuilding. |
| `imageWorkdir` | `"/app"` | Image working directory sources are synced to (moon's Dockerfiles use `/app`). |
| `dockerBin` | `"docker"` | Binary used to build images. |
| `moonBin` | `"moon"` | moon binary used by resources. |
| `tiltBin` | `"tilt"` | Tilt binary. With the default and the `tilt` toolchain configured, the binary the toolchain installed for its pinned version is used (run `moon setup` if it isn't installed yet), regardless of `PATH` or `tilt` plugins in `.prototools`. Any other value is used as-is. |

### Project settings

In a project's `moon.yml`, under `project.tilt` (moon's custom project metadata):

| Setting | Description |
| --- | --- |
| `serviceRestart` | Overrides the workspace `serviceRestart` for this project (local services and Live Update restarts). |
| `resources` | Extra Tilt resource names (e.g. from the project's own `Tiltfile`) that the project ID selects. |
| `workloads` | Tilt resources the project's own Tiltfile deploys, by kind: `k8s` (YAML, kustomize, Helm, `helm_resource`) and `compose`. Containerizes the project. See [Workloads your Tiltfiles deploy](#workloads-your-tiltfiles-deploy). |
| `image` | Build this project's image from moon's Dockerfile under this name (used verbatim, registry prefix included). Your own resources deploy it. |
| `liveUpdate` | `false` rebuilds the image on every change instead of syncing. |
