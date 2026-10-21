# Tasks

## 1. Resolve the installed Tilt binary

- [x] 1.1 Add a pure function that picks the highest installed version satisfying a version spec (exact, partial, requirement, range, `latest`/`stable` aliases), reading `installed_versions` from proto's tool manifest JSON, and add `version_spec` as a direct dependency; verify with unit tests for each spec form, no match, a missing or malformed manifest, and canary
- [x] 1.2 Build the Tilt command from it: an explicit non-default `tiltBin` as-is; else, with the toolchain configured and a version, `/proto/tools/tilt/<version>/tilt` with that directory first on the child `PATH`; else `tilt` from `PATH`. Drop `PROTO_TILT_VERSION` and the shim paths, and fail with an error naming the requested version, the installed versions, and `moon setup` when nothing matches. Verify with unit tests for each branch and the error text, and that `moon ext tilt -- up/focus/ci/down` and `set_args`/`running_args` all go through it

## 2. Documentation

- [x] 2.1 Update `toolchains/tilt/README.md`: pinning in `.prototools` (moon's `versionFromPrototools`, precedence of `version`), that the extension runs the toolchain's installed binary, `moon setup` when it isn't installed, and how a `tilt` plugin in `.prototools` interacts (shell only, shared installs, `proto trust` on proto 0.63+); verify the examples match what the scratch-workspace check in 3.1 runs
- [x] 2.2 Update the extension README's toolchain notes (the `tiltBin` setting and requirements), and verify no README still mentions `PROTO_TILT_VERSION` or shims for the extension

## 3. Verification

- [x] 3.1 With real moon 2.6.2 and a scratch `PROTO_HOME`, in a scratch workspace whose `.prototools` pins `tilt` with no `[plugins]` entry and with Tilt not on `PATH`: verify `moon ext tilt -- down` runs the toolchain's Tilt; that it fails with the `moon setup` error when the pinned version isn't installed, and works after `moon setup`; and that a registered (trusted) `.prototools` plugin doesn't change which binary the extension runs
- [x] 3.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), merging the delta into `openspec/specs/tilt-toolchain`.
- Commit as `fix(tilt): run the toolchain's installed Tilt instead of relying on PATH and proto shims` on `fix/run-installed-tilt`, and open a PR (main is protected).
- After the PR merges, reply on issue #3 with the findings (versionFromPrototools already works, and the extension no longer uses `.prototools` plugins) and close it.
