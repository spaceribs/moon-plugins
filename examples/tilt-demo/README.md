# tilt-demo

A small standalone moon workspace for trying the Tilt plugins by hand. It has a `web` frontend, an `api` and `db`, a shared `api-client` library, and a `docs` project with its own Tiltfile.

It uses **locally built** plugins. To try the **published** ones, swap the `plugin:` lines in `.moon/extensions.yml` and `.moon/toolchains.yml` for the `github://` locators noted there, and skip the build step.

```sh
# from the repo root (local plugin builds only)
moon run tilt-extension:build tilt-toolchain:build

cd examples/tilt-demo
moon sync                     # writes .moon/tilt/ + Tiltfile (also happens before any `moon run`)

# Start the single Tilt instance, either with everything...
moon ext tilt -- up           # (same as `tilt up`): db:serve → api:serve → web:dev, docs, ...
# ...or scoped to a project and everything it needs:
moon ext tilt -- up web

# Then, from another terminal, change what that instance runs:
moon ext tilt -- up docs      # add docs
moon ext tilt -- focus api    # only api (and db) from now on
```

**What it shows:**

- `web` is tagged `tilt`, so its persistent `dev` task is exposed, along with its `codegen` dependency.
- `api:serve` is tagged individually.
- `web` → `api-client` (library) → `api` → `db` in the project graph. So `web:dev` waits on `api:serve`, which waits on `db:serve`. `db:serve` is included even though it isn't tagged.
- `web` → `tooling` is a `build`-scoped dependency, so `tooling:watch` is not exposed.
- `web:dev` runs `node --watch`, which reloads itself, so editing `web/src` restarts nothing in Tilt. Editing `web/schema.json` re-runs `web:codegen`, and `web:dev` restarts because codegen's outputs changed.
- `api:serve` is a plain `node` server, so `api/moon.yml` sets `project.tilt.serviceRestart: "inputs"`.
- `docs` has its own `Tiltfile`, which the generated entry file (`.moon/tilt/Tiltfile`) `include()`s. Its `docs` resource is selected by name.
- `web:repl` is tagged but interactive, so it's never exposed.

moon requires a git repository with at least one commit. After rebuilding a plugin, delete `.moon/cache` here.

## Trying containers

The demo runs everything on the host. To run `api` on Kubernetes instead, you need Docker and a local cluster (e.g. `ctlptl create cluster kind --registry=ctlptl-registry`). Write its `Dockerfile` with `moon docker file api`, add a manifest, and opt the project in:

```yaml
# api/moon.yml
docker:
  file:
    image: "node:22"
    startTask: "serve"
project:
  tilt:
    serviceRestart: "inputs"
    image: "api"
    workloads: { k8s: ["api"] }
```

```python
# api/Tiltfile
k8s_yaml("k8s.yaml")   # a Deployment (and Service) using image "api"
k8s_resource("api", port_forwards=[3001])
```

Then `moon ext tilt -- up api` builds the image, deploys it, and runs `db:serve` alongside. `api:serve` no longer runs on the host, `web:dev` waits on the `api` pod, and editing `api/src` syncs into the pod and restarts the server instead of rebuilding.

moon runs inside the image build too, so the extension must load there. This workspace points at `../../../target`, outside the build context, so vendor the `.wasm` into the workspace first. See [Plugins inside the image build](../../extensions/tilt/README.md#plugins-inside-the-image-build).
