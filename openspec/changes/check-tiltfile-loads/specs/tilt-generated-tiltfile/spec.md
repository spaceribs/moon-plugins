# Spec Delta

## ADDED Requirements

### Requirement: Load check
`moon ext tilt -- generate --load` SHALL load the main Tiltfile as Tilt does, without a cluster or a running Tilt, after generating. When it loads, the command SHALL succeed without printing Tilt's evaluation result. When it doesn't, the command SHALL fail with Tilt's error. With `--check`, it SHALL load only when the output is current, writing nothing.

#### Scenario: Workload no Tiltfile deploys
- **WHEN** `api` declares `workloads: { k8s: ["api"] }` but no Tiltfile deploys an `api` resource, and CI runs `moon ext tilt -- generate --load`
- **THEN** the command fails with Tilt's `k8s_resource specified unknown resource "api"` error, without connecting to a cluster

#### Scenario: Tiltfile loads
- **WHEN** every declared workload is deployed and CI runs `moon ext tilt -- generate --load`
- **THEN** the command succeeds and prints no JSON

#### Scenario: Stale output with check
- **WHEN** a task was added since the output was last generated, and CI runs `moon ext tilt -- generate --check --load`
- **THEN** the command fails naming `resources.tilt`, without loading or writing anything

#### Scenario: Tilt missing
- **WHEN** neither `tiltBin` nor the Tilt toolchain's binary is available
- **THEN** `generate --load` fails with an error naming the missing binary
