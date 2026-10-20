# moon-plugins

Custom [WASM plugins](https://moonrepo.dev/docs/guides/wasm-plugins) for [moon](https://moonrepo.dev/moon) and [proto](https://moonrepo.dev/proto), built, tested, and released from a moon workspace.

| Plugin | Kind | Crate | Description |
| --- | --- | --- | --- |
| [Tilt](./extensions/tilt) | moon extension | `tilt_extension` | Exposes the moon task graph as Tilt resources (`moon ext tilt -- up`). |
| [Tilt](./toolchains/tilt) | moon toolchain | `tilt_toolchain` | Installs and pins Tilt for tasks and the extension. |
| [Tilt](./tools/tilt) | proto tool | `tilt_tool` | `proto install tilt`. Also bundled into `tilt_toolchain`. |

## Using the plugins

Reference a plugin with a `github://` locator. moon downloads its `.wasm` from this repo's GitHub releases:

```yaml
# .moon/extensions.yml
tilt:
  plugin: "github://spaceribs/moon-plugins/tilt_extension"                               # latest
  # plugin: "github://spaceribs/moon-plugins/tilt_extension@tilt_extension-v0.1.0"       # pinned (full release tag)
```

Each plugin's README covers setup: start with the [Tilt extension's quick start](./extensions/tilt#quick-start).

**Supported platforms:** Linux and macOS. The proto plugin (`tilt_tool`) also installs Tilt on Windows, but the extension's container builds need `sh`, and CI doesn't run on Windows.

## Layout

```
extensions/<name>/   moon extensions   (moon ext <id>)
toolchains/<name>/   moon toolchains   (.moon/toolchains.yml)
tools/<name>/        proto tools       (.prototools [plugins.tools])
examples/<name>/     standalone moon workspaces for manual end-to-end testing
```

Each plugin is a `cdylib` crate compiled to `wasm32-wasip1`. Pure logic lives in modules that compile natively. The WASM entry points are behind the default `wasm` feature.

## Development

You need [proto](https://moonrepo.dev/docs/proto/install) **0.60 or newer** (moon 2.6 requires it) and rustup. moon and Rust are pinned in `.prototools` and `rust-toolchain.toml`.

```sh
proto install                     # moon + Rust (incl. the wasm32-wasip1 target)
moon run :build                   # release .wasm builds (used by tests and dogfooding)
moon run :test                    # native unit tests + WASM integration tests
moon run :lint root:format        # clippy (-D warnings) + rustfmt
```

Integration tests load the built `.wasm` into a sandboxed host via `moon_pdk_test_utils` / `proto_pdk_test_utils`, which is why `test` depends on `build`. The harness looks in `target/wasm32-wasip1/release` before `debug`. So if you run `cargo test` by hand, rebuild with `--release` first, or a stale release build will be tested. Some tool tests download real Tilt releases.

### Trying a plugin locally

Point a workspace at the built file with a `file://` locator. For example, [`examples/tilt-demo`](./examples/tilt-demo) does this:

```yaml
# .moon/extensions.yml
tilt:
  plugin: "file://../../../target/wasm32-wasip1/release/tilt_extension.wasm"
```

`file://` paths are for local development only. They must point inside the consuming workspace if moon also runs inside container builds (see the Tilt extension's container docs). moon caches the project graph, so after rebuilding a plugin, delete `.moon/cache` in the consuming workspace.

### Adding a plugin

1. Create `extensions/<name>`, `toolchains/<name>`, or `tools/<name>`. Copy the Tilt crate as a template: `crate-type = ["cdylib", "lib"]` and a `wasm` default feature.
2. Register it in `.moon/workspace.yml`. Cargo picks it up automatically through the workspace globs.
3. Add a `CHANGELOG.md` with a `# Changelog` heading. release-plz adds a section for each release.

## Spec-driven development (OpenSpec)

Changes are planned with [OpenSpec](https://github.com/Fission-AI/OpenSpec) before they're built. Specs for the plugins' behavior live in [`openspec/specs/`](./openspec/specs), and proposed changes in [`openspec/changes/`](./openspec/changes).

OpenSpec is pinned in `.prototools` and installed by `proto install`, through proto's npm backend (needs proto 0.57+, so run `proto upgrade` first if needed). That puts `openspec` on `PATH` for you, for AI assistants, and for moon.

With Claude Code (instructions in [`.claude/`](./.claude)):

| Command | What it does |
| --- | --- |
| `/opsx:explore` | think an idea through before committing to it |
| `/opsx:propose <idea>` | create a change: proposal, spec deltas, design, and tasks (planning only) |
| `/opsx:apply` | implement the change's tasks |
| `/opsx:archive` | merge the change's spec deltas into `openspec/specs/` and archive it |

With moon:

```sh
moon run root:openspec          # validate all specs and changes (strict), also run in CI
moon run root:openspec-update   # regenerate .claude/ instructions after upgrading OpenSpec
openspec list                   # changes in progress (or --specs)
openspec view                   # interactive dashboard
```

Project-wide guidance for AI-written artifacts is in [`openspec/config.yaml`](./openspec/config.yaml). To upgrade OpenSpec, bump its version in `.prototools`, then run `proto install` and `moon run root:openspec-update`. OpenSpec collects anonymous usage telemetry outside CI; opt out with `openspec config set telemetry.enabled false`.

## Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org), because they decide each plugin's next version and changelog:

```
<type>(<scope>)!: <subject>

feat(tilt): add readiness probes
fix(tilt_tool): handle musl on arm
docs: explain Live Update
feat(tilt)!: make container deploys opt-in
```

| Type | Releases? |
| --- | --- |
| `feat` | minor version (0.1.0 → 0.2.0) |
| `fix`, `perf` | patch version (0.1.0 → 0.1.1) |
| any type with `!` (or a `BREAKING CHANGE:` footer) | breaking change |
| `docs`, `chore`, `ci`, `build`, `refactor`, `style`, `test`, `revert` | no release on their own |

A commit releases the plugins whose files it changes. The scope is free-form, but naming the plugin (`tilt`, `tilt_tool`, `tilt_toolchain`) keeps the changelogs readable. Changes to `tilt_tool` also release `tilt_toolchain`, which bundles it.

Messages are checked by a `commit-msg` hook ([`scripts/commit-lint.sh`](./scripts/commit-lint.sh)). moon installs it whenever it runs a task, or run `moon sync hooks` once. CI checks every commit in pull requests and pushes too.

## Releasing

Releases are automated with [release-plz](https://release-plz.dev) ([`release-plz.toml`](./release-plz.toml)):

1. **On every push to `main`**, release-plz opens or updates a **release PR**. It bumps the version and changelog of every plugin with releasable commits since its last release.
2. **Merging the release PR** tags each bumped plugin (`<crate>-v<version>`, e.g. `tilt_extension-v0.2.0`), then dispatches [`release.yml`](./.github/workflows/release.yml) for each tag.
3. **`release.yml`** uses [`moonrepo/build-wasm-plugin`](https://github.com/moonrepo/build-wasm-plugin) to build, `wasm-opt`, strip, and checksum the plugin. It creates the GitHub release with the `.wasm` attached and the changelog entry as notes, and publishes to ghcr.io.

The release PR is opened with GitHub Actions' token, so CI doesn't run on it automatically. It only changes versions and changelogs, which were already tested on `main`. To release by hand instead, push a tag yourself (`git tag tilt_tool-v0.1.2 && git push origin tilt_tool-v0.1.2`) after bumping the version and changelog.

Consumers then reference it with a `github://` locator ([see above](#using-the-plugins)). A pinned locator uses the full release tag (`tilt_extension@tilt_extension-v0.1.0`), because moon looks the tag up verbatim.
