# Design

## Context

- `plan_images` (`images.rs`) builds each image's `command`, a shell string for Tilt's `custom_build`: `{dockerBin} build -t $EXPECTED_REF -f {dockerfile} .`. Tilt runs it with `sh -c`, and expands `$EXPECTED_REF` there. Image builds are supported on Linux and macOS (including WSL2) only.
- Per-project image settings are read from `project.tilt` through `tilt_metadata()`. Malformed `liveUpdate` settings are reported before planning by `live_update_settings_error`, which `render_generated` calls with the other validation passes.

## Goals / Non-Goals

**Goals:**
- Private-dependency builds work without leaving the extension's image support.
- The generated command stays readable and shell-safe.

**Non-Goals:**
- A general passthrough of arbitrary `docker build` flags.

## Decisions

**Named options, not a raw flags list.** The issue's four options cover the private-dependency case and the common neighbours (a build stage, build args). Named options can be validated and quoted, and they stay meaningful if the build command changes (say, to `docker buildx build`). A raw `flags: [...]` list would also pass anything that breaks the context or tag the extension relies on (`-f`, `-t`, a second context).

**Fixed order: `--target`, `--build-arg` (sorted by name), `--ssh`, `--secret`, in list order, before the context `.`.** Stable output means unchanged settings never rewrite the generated file.

**Shell quoting.** A value made only of characters safe in `sh` (`A-Za-z0-9` and `_./:=,@%+-`) is written as is. Anything else is single-quoted, with `'` written as `'\''`. Quoting applies to the whole `NAME=value` word of a build arg. This keeps `--ssh default` and `--secret id=npmrc,src=.npmrc` readable while making `github.com/acme/*` literal.

**Validation in `image_build_settings_error`,** next to `live_update_settings_error`, called from `render_generated` before planning. serde with `deny_unknown_fields` rejects unknown options and wrong types. Explicit checks cover empty strings, build-arg names, and a project without `project.tilt.image` (the options would otherwise be ignored silently). Secret and SSH values aren't parsed further: Docker validates them, and their syntax has grown over versions.

**No rebuild trigger for secret files.** A `--secret src=` file is often outside the workspace or gitignored, and its content shouldn't enter the build's cache key anyway. Graph-derived triggers are unchanged.

## Risks / Trade-offs

- [Build args are recorded in the image's history] → The README says to pass credentials with `secrets` or `ssh`, never `args`.
- [`--ssh default` lets any build step use the developer's keys] → Per project and opt-in, as the issue asks. The README says to enable it only where a step needs it.
- [An older Docker without BuildKit by default] → `--ssh`/`--secret` fail with Docker's own error. The README notes BuildKit (default since Docker 23).

## Migration Plan

Opt-in. A project that dropped `project.tilt.image` for its own `docker_build(..., ssh='default')` can restore `image` with `imageBuild: { ssh: ["default"] }` and delete its own build. Release as `feat(tilt)` (minor).
