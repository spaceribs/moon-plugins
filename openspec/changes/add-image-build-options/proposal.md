# Proposal

## Why

Issue #49: an image is built with `docker build -t $EXPECTED_REF -f <project>/Dockerfile .`, with no way to add options. A project whose build fetches private dependencies over git (Go modules, Python git sources) needs the build to forward an SSH agent: a `RUN --mount=type=ssh` step, built with `docker build --ssh default`. Without the option the build fails at that step, and the only workaround is dropping `project.tilt.image` for a hand-written `docker_build(..., ssh='default')`, which loses the graph-derived rebuild triggers and Live Update.

## What Changes

- A per-project setting, `project.tilt.imageBuild`, whose options are added to the project's generated `docker build`:
  - `ssh`: a list, each passed as `--ssh <value>` (`["default"]` → `--ssh default`)
  - `args`: an object, each passed as `--build-arg NAME=VALUE`
  - `target`: a string, passed as `--target <value>`
  - `secrets`: a list, each passed as `--secret <value>` (`"id=npmrc,src=.npmrc"`)
- Values are shell-quoted in the generated command where needed.
- Invalid settings fail generation, naming the project and the option, as do settings on a project that doesn't set `project.tilt.image`.
- Per project, since only projects with private dependencies should forward an agent: each use can prompt the developer's key agent.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-container-images`: build options from `project.tilt.imageBuild`.

## Impact

- **Code**: `extensions/tilt/src/images.rs` (parsing, validation, the command), `tilt_ext.rs` (reporting the validation error), the README's images and project-settings sections, and `GLOSSARY.md` (image build options).
- **Behavior**: opt-in. Without the setting, the build command is unchanged. Released as `feat(tilt)` (minor).
- **Dependencies**: `--ssh` and `--secret` need BuildKit, Docker's default builder since Docker 23. Image builds already require Linux or macOS (including WSL2), where the command runs through `sh`.
- **Out of scope**: other `docker build` flags (`--platform`, `--cache-from`, ...), and build options for images the user's own Tiltfile builds.
