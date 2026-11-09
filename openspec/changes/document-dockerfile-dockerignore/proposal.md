# Proposal

## Why

Issue #50: the README's Images section recommends a workspace-root `.dockerignore`, since every build sends the whole workspace. BuildKit also reads a Dockerfile-specific ignore file, `<path>/Dockerfile.dockerignore`, in preference to the root one when building with `-f <path>/Dockerfile`, which is what the extension's build command does. A workspace can use it to narrow each image's context to what that project's graph needs, so other projects' files never enter the build. Nothing tells users this works.

## What Changes

- A paragraph under Images: `<project>/Dockerfile.dockerignore` works with the extension's builds and takes precedence over the root `.dockerignore`, it needs BuildKit, and the extension's graph-derived rebuild triggers are unaffected by it (it narrows what Docker receives, not what Tilt watches).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. Documentation only (`skip_specs: true`): the build command and rebuild triggers don't change.

## Impact

- **Docs**: the extension README's Images section.
- **Behavior**: none. Released as `docs(tilt)`.
