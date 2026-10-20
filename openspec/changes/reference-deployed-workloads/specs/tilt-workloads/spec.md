# Spec Delta

## Purpose

Lets a project declare, by reference, the Kubernetes or Compose workloads its own Tiltfile deploys, so they join the project graph, selection, and labels without the extension generating any deployment.

## ADDED Requirements

### Requirement: Projects declare workloads by kind and name
A project SHALL be able to declare the Tilt resources its own Tiltfiles deploy, under `project.tilt.workloads`, as lists of names per kind: `k8s` for Kubernetes workloads (including kustomize and Helm charts rendered with `helm()`) and `compose` for Docker Compose services. The extension MUST NOT generate the deployment itself.

#### Scenario: Kubernetes workloads
- **WHEN** `api/moon.yml` sets `project.tilt.workloads: { k8s: ["api", "api-worker"] }` and `api/Tiltfile` deploys both with `k8s_yaml`
- **THEN** the generated Tiltfile references `api` and `api-worker` as Kubernetes resources and contains no manifest for them

#### Scenario: Compose service
- **WHEN** a project sets `project.tilt.workloads: { compose: ["api"] }` and its Tiltfile calls `docker_compose`
- **THEN** the generated Tiltfile references `api` as a Compose resource

### Requirement: Declared workloads containerize the project
A project with at least one declared workload SHALL be containerized: its tasks MUST NOT run as local resources, even when the project or its tasks are tagged.

#### Scenario: Tagged project with a workload
- **WHEN** project `api` is tagged `tilt`, has a persistent task `serve`, and declares workload `api`
- **THEN** `api:serve` is not a local resource, and the `api` workload is

### Requirement: Workloads join the project graph
Each declared workload SHALL get `resource_deps` on the upstream services of its project, including other projects' declared workloads, and those services SHALL be included. Services in projects that depend on a containerized project SHALL wait on all of its declared workloads.

#### Scenario: Downstream service waits on the workload
- **WHEN** `web` depends on `api` through the project graph, and `api` declares workloads `api` and `api-worker`
- **THEN** `web:dev` has `resource_deps` on `api` and `api-worker`

#### Scenario: Workload waits on an upstream service
- **WHEN** `api` declares workload `api` and depends on project `db` with service `db:serve`
- **THEN** the `api` workload has `resource_deps` on `db:serve`, and `db:serve` is included

### Requirement: Decorations never replace project settings
The extension SHALL add its project label and `resource_deps` to declared workloads without removing labels, dependencies, port-forwards, or other settings that the project's own Tiltfile gives them, regardless of the order in which the Tiltfiles are evaluated.

#### Scenario: Project sets its own labels and port-forwards
- **WHEN** `api/Tiltfile` calls `k8s_resource("api", labels=["team-a"], port_forwards=[3001])`
- **THEN** the `api` resource keeps label `team-a` and port-forward 3001, and also has label `api` and the extension's `resource_deps`

### Requirement: Undeclared or misnamed workloads fail loudly
When a declared workload name doesn't match any resource of the declared kind, loading the Tiltfile MUST fail with an error naming the missing resource.

#### Scenario: Typo in a workload name
- **WHEN** a project declares `k8s: ["apii"]` but its Tiltfile deploys `api`
- **THEN** Tilt reports an unknown resource `apii` when loading the Tiltfile
