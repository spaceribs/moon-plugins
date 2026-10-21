# Tasks

## 1. Run resources with moon exec

- [x] 1.1 Change the resource command to `moon exec <target> --upstream none --on-failure bail --ignore-ci-checks` (parameterized variants still append `-- <args>`), and update the unit and render tests that assert the command; verify with `cargo test -p tilt_extension --no-default-features --lib`, including the persistent-task render and the parameterized-dependency resource
- [x] 1.2 Update the extension README's description of how resources run (and any other mention of the resource command), and verify no doc still says resources run `moon run`

## 2. Verification

- [x] 2.1 With real moon 2.6.2 on `examples/tilt-demo`, verify that `CI=true moon ext tilt -- ci web` brings up `db:serve`, `api:serve`, and `web:dev` (it failed with `No tasks found` before), and that it also passes without `CI`
- [x] 2.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), merging the delta into `openspec/specs/tilt-resources`.
- Commit as `fix(tilt): run resources with moon exec so persistent tasks run in CI`, push `fix/run-resources-in-ci`, and open a PR.
- After it merges, rebase the Windows PR (#11) onto `main`, and add a `.gitattributes` forcing LF there (its Windows unit tests failed on CRLF templates).
