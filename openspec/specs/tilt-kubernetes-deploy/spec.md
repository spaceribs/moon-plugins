# tilt-kubernetes-deploy Specification

## Purpose
How projects migrate off the removed `project.tilt.k8s` manifest generation: the error that points them to migration, and the `export-k8s` command that writes their former manifest for them to deploy themselves (see `tilt-workloads`).

## Requirements

### Requirement: Removed k8s settings fail with migration guidance
When a project still sets `project.tilt.k8s`, generation MUST fail with an error naming the project, stating that manifest generation was removed, and pointing to `moon ext tilt -- export-k8s <project>` and `project.tilt.workloads`.

#### Scenario: Leftover k8s settings
- **WHEN** `api/moon.yml` sets `project.tilt.k8s: { ports: [3001] }`
- **THEN** `moon ext tilt -- generate` fails with an error naming `api`, `export-k8s`, and `project.tilt.workloads`

### Requirement: Export former k8s settings as a manifest
`moon ext tilt -- export-k8s <project>` SHALL write a Kubernetes manifest equivalent to what the removed generator produced from the project's `project.tilt.k8s` settings (Deployment, a Service for ports, and the readiness probe) to `<project>/k8s.yaml`, or to the path given with `--output <path>`, MUST NOT overwrite an existing file, and SHALL print the `project.tilt.workloads` and Tiltfile lines needed to deploy it.

#### Scenario: Migrating a project
- **WHEN** the user runs `moon ext tilt -- export-k8s api` for a project with `project.tilt.k8s: { ports: [3001] }`
- **THEN** `api/k8s.yaml` contains the Deployment and Service, and the command prints `workloads: { k8s: ["api"] }` and `k8s_yaml("k8s.yaml")` with `k8s_resource("api", port_forwards=[3001])`

#### Scenario: Custom output path
- **WHEN** the user runs `moon ext tilt -- export-k8s api --output deploy/k8s/api.yaml`
- **THEN** the manifest is written to `deploy/k8s/api.yaml`, and the printed `k8s_yaml(...)` line references that file

#### Scenario: File already exists
- **WHEN** `api/k8s.yaml` already exists
- **THEN** the command fails without changing it
