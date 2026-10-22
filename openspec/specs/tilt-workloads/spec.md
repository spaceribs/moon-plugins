# tilt-workloads Specification

## Purpose
Lets a project declare, by reference, the Kubernetes or Compose workloads its own Tiltfile deploys, so they join the project graph, selection, and labels without the extension generating any deployment.

## Requirements

### Requirement: Projects declare workloads by kind and name
A project SHALL be able to declare the Tilt resources its own Tiltfiles deploy, under `project.tilt.workloads`, as lists of names per kind: `k8s` for Kubernetes workloads (including kustomize, Helm charts rendered with `helm()`, and Tilt's `helm_resource` extension) and `compose` for Docker Compose services. The extension MUST NOT generate the deployment itself.

#### Scenario: Kubernetes workloads
- **WHEN** `api/moon.yml` sets `project.tilt.workloads: { k8s: ["api", "api-worker"] }` and `api/Tiltfile` deploys both with `k8s_yaml`
- **THEN** the generated Tiltfile references `api` and `api-worker` as Kubernetes resources and contains no manifest for them

#### Scenario: Helm release deployed with helm_resource
- **WHEN** `sre/grafana/moon.yml` sets `project.tilt.workloads: { k8s: ["grafana", "pushgateway"] }` and its Tiltfile deploys both with `helm_resource`
- **THEN** the generated Tiltfile references `grafana` and `pushgateway` as Kubernetes resources, and Tilt applies the decorations to them

#### Scenario: Compose service
- **WHEN** a project sets `project.tilt.workloads: { compose: ["api"] }` and its Tiltfile calls `docker_compose`
- **THEN** the generated Tiltfile references `api` as a Compose resource

### Requirement: Declared workloads containerize the project
A project with at least one declared workload SHALL be containerized: its persistent tasks, and any task selected only through the project's tag or `includePersistent`, MUST NOT run as local resources. A non-persistent task that carries the tag itself SHALL still be a local resource, along with its non-persistent task dependencies.

#### Scenario: Tagged project with a workload
- **WHEN** project `api` is tagged `tilt`, has a persistent task `serve`, and declares workload `api`
- **THEN** `api:serve` is not a local resource, and the `api` workload is

#### Scenario: Individually tagged persistent task
- **WHEN** a containerized project's persistent task `serve` carries the tag itself
- **THEN** `serve` is still not a local resource, since the workload replaces it

#### Scenario: Task-tagged build step in a containerized project
- **WHEN** containerized project `django-api` has a non-persistent task `schema-export` that carries the tag itself
- **THEN** `django-api:schema-export` is a local resource, while `django-api`'s persistent tasks are not

### Requirement: Workloads join the project graph
Each declared workload SHALL get `resource_deps` on the upstream services of its project, including other projects' declared workloads, and those services SHALL be included. Services in projects that depend on a containerized project SHALL wait on all of its declared workloads.

#### Scenario: Downstream service waits on the workload
- **WHEN** `web` depends on `api` through the project graph, and `api` declares workloads `api` and `api-worker`
- **THEN** `web:dev` has `resource_deps` on `api` and `api-worker`

#### Scenario: Workload waits on an upstream service
- **WHEN** `api` declares workload `api` and depends on project `db` with service `db:serve`
- **THEN** the `api` workload has `resource_deps` on `db:serve`, and `db:serve` is included

### Requirement: Decorations never replace project settings
The extension SHALL add `resource_deps` (and, per `workloadLabels`, a project label) to declared workloads without removing labels, dependencies, port-forwards, or other settings that the project's own Tiltfile gives them. For Kubernetes workloads this SHALL hold regardless of the order in which the Tiltfiles are evaluated. Because Tilt requires Compose services to be loaded before `dc_resource`, the generated Tiltfile SHALL include project Tiltfiles before its decorations.

#### Scenario: Project sets its own labels and port-forwards
- **WHEN** `api/Tiltfile` calls `k8s_resource("api", labels=["team-a"], port_forwards=[3001])`
- **THEN** the `api` resource keeps label `team-a` and port-forward 3001, and also has label `api` and the extension's `resource_deps`

#### Scenario: Decorations evaluated before the Kubernetes deploy
- **WHEN** the user's main Tiltfile includes the generated file first, then calls `k8s_yaml` and `k8s_resource("api", labels=["team-a"])`
- **THEN** the `api` resource has labels `api` and `team-a`, and the extension's `resource_deps`

#### Scenario: Compose service loaded by a project Tiltfile
- **WHEN** `api/Tiltfile` calls `docker_compose` and `dc_resource("cache", labels=["team-a"])`, and the project declares `compose: ["cache"]`
- **THEN** the `cache` resource has labels `api` and `team-a`, and the extension's `resource_deps`

### Requirement: Workload labels are configurable
The workspace setting `workloadLabels` SHALL control the label the extension adds to declared workloads: `project` (default) adds the project ID as a label, and `none` adds no label, leaving the project's own labels as the only ones.

#### Scenario: Workloads already labelled by role
- **WHEN** `workloadLabels` is `none` and `django-api/Tiltfile` labels its workload `server`
- **THEN** the `django-api` workload has only the label `server`, and still gets the extension's `resource_deps`

### Requirement: Undeclared or misnamed workloads fail loudly
When a declared workload name doesn't match any resource of the declared kind, loading the Tiltfile MUST fail with an error naming the missing resource.

#### Scenario: Typo in a workload name
- **WHEN** a project declares `k8s: ["apii"]` but its Tiltfile deploys `api`
- **THEN** Tilt reports an unknown resource `apii` when loading the Tiltfile

### Requirement: Project Tiltfiles can label resources like workloads
The generated `lib.star` SHALL export `moon_labels(project)`, returning the labels the extension gives that project's declared workloads: `[project]` under `workloadLabels: "project"` and `[]` under `"none"`. A project Tiltfile SHALL be able to `load()` it by a path relative to itself, so resources it defines group with the project's workloads in Tilt's UI.

#### Scenario: Routes step grouped with its project
- **WHEN** `ingress/Tiltfile` loads `../.moon/tilt/lib.star` and defines `local_resource("ingress-routes", …, labels = moon_labels("ingress"))`
- **THEN** `ingress-routes` has the label `ingress`, like the `ingress` workload

#### Scenario: Labels turned off
- **WHEN** `workloadLabels` is `none` and a project Tiltfile passes `labels = moon_labels("ingress")`
- **THEN** the resource gets no label from the helper

### Requirement: Unknown projects fail label lookup
`moon_labels` SHALL fail Tiltfile loading, with an error naming the ID, when given an ID that isn't a moon project, whatever `workloadLabels` is.

#### Scenario: Typo in a project ID
- **WHEN** a project Tiltfile calls `moon_labels("ingres")`
- **THEN** Tilt reports an error naming `ingres` when loading the Tiltfile
