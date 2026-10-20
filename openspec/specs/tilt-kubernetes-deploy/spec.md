# tilt-kubernetes-deploy Specification

## Purpose
How projects that opt in with `project.tilt.k8s` are deployed to a Kubernetes cluster and fit into the Tilt dependency graph in place of their local tasks.

## Requirements

### Requirement: Generated Deployment and Service
A project with `project.tilt.k8s` SHALL get a Deployment named after the project, running its image with the configured `replicas` (default 1), `env`, and container `resources`, plus a Service exposing each configured port. The manifests SHALL reference the image by the same name the image build produces.

#### Scenario: Ports configured
- **WHEN** `project.tilt.k8s.ports` is `[3001]`
- **THEN** a Service named after the project exposes port 3001
- **AND** Tilt port-forwards 3001 to localhost

#### Scenario: No ports
- **WHEN** no ports are configured
- **THEN** no Service is generated

### Requirement: Readiness probe
With ports configured, the container SHALL get a TCP readiness probe on the first port by default, so Tilt reports it healthy only once it accepts connections. A custom `readiness` probe SHALL be passed through, and `readiness: false` SHALL disable it.

#### Scenario: Slow-starting app
- **WHEN** the container starts but the app isn't listening yet
- **THEN** Tilt doesn't report the workload healthy

### Requirement: Containerized projects replace their local tasks
A deployed project's tasks MUST NOT run as local resources. Its Kubernetes resource, named after the project, SHALL stand in as its service in the project graph: dependents wait on it, it waits on its own upstream services (which are included), and its project ID selects it.

#### Scenario: Dependent local service
- **WHEN** `api` is deployed and `web:dev` depends on `api` through the project graph
- **THEN** `api:serve` is not a local resource, and `web:dev` waits on the `api` workload
