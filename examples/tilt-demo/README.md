# tilt-demo

A small standalone moon workspace that shows off the Tilt plugins: a `web` frontend, an `api` and `db`, a shared `api-client` library, and a `docs` project with its own Tiltfile.

It's set up to test **locally built** plugins. To try the **published** plugins instead, swap the `plugin:` lines in `.moon/extensions.yml` and `.moon/toolchains.yml` for the `github://` locators noted there, and skip the build step.

```sh
# from the repo root (local plugin builds only)
moon run tilt-extension:build tilt-toolchain:build

cd examples/tilt-demo
moon sync                     # writes Tiltfile.moon + Tiltfile (also happens before any `moon run`)

# Start the single Tilt instance, either with everything...
moon ext tilt -- up           # (same as `tilt up`): db:serve → api:serve → web:dev, docs, ...
# ...or scoped to a project and everything it needs:
moon ext tilt -- up web

# Then, from another terminal, change what that instance runs:
moon ext tilt -- up docs      # add docs
moon ext tilt -- focus api    # only api (and db) from now on
```

To run a project on Kubernetes, you need Docker and a local cluster (e.g. `ctlptl create cluster kind --registry=ctlptl-registry`). Add `docker.file` settings and ports to the project's `moon.yml`, e.g. for `api`:

```yaml
docker:
  file:
    image: "node:22"
    startTask: "serve"
project:
  tilt:
    k8s:
      ports: [3001]
      resources:
        limits: { memory: "768Mi" }
```

Then `moon ext tilt -- ci api` builds the image, deploys it, and waits until it accepts connections. moon runs inside the image build too, so the extension must be loadable there. This workspace points at `../../../target`, outside the build context, so vendor the `.wasm` into the workspace first. See [Plugins inside the image build](../../extensions/tilt/README.md#plugins-inside-the-image-build).

- `web` is tagged `tilt`, so its persistent `dev` task is exposed, along with its `codegen` dependency.
- `api:serve` is tagged individually.
- `web` → `api-client` (library) → `api` → `db` in the project graph. So `web:dev` waits on `api:serve`, which waits on `db:serve`. `db:serve` is included even though it isn't tagged.
- `web` → `tooling` is a `build`-scoped dependency, so `tooling:watch` is not exposed.
- `web:dev` runs `node --watch`, which reloads itself, so editing `web/src` restarts nothing in Tilt. Editing `web/schema.json` re-runs `web:codegen`, and `web:dev` restarts because codegen's outputs changed.
- `api:serve` is a plain `node` server, so `api/moon.yml` sets `project.tilt.serviceRestart: "inputs"`.
- `docs` has its own `Tiltfile`, which the generated file `include()`s. Its `docs` resource is selected by name.

moon requires a git repository with at least one commit. After rebuilding a plugin, delete `.moon/cache` here.
