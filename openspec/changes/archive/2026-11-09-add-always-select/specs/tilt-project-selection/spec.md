# Spec Delta

## ADDED Requirements

### Requirement: Always-selected projects and resources
Each entry in the workspace setting `alwaysSelect` (default `[]`) SHALL be added to every non-empty selection, as if it had been given as a Tiltfile arg too: a project ID adds that project's selection, and any other value is a resource name. With no Tiltfile args, everything SHALL stay enabled. With `selectByProject` disabled, `alwaysSelect` SHALL have no effect.

#### Scenario: Observability with every selection
- **WHEN** `alwaysSelect` is `["observability"]`, project `observability` declares `workloads: { k8s: ["grafana", "otel-collector"] }`, and Tilt runs with args `api`
- **THEN** `api`'s selection, `grafana`, and `otel-collector` are enabled

#### Scenario: Resource name
- **WHEN** `alwaysSelect` is `["ingress-dashboards"]`, which isn't a project ID, and Tilt runs with args `web`
- **THEN** `web`'s selection and the `ingress-dashboards` resource are enabled

#### Scenario: No selection
- **WHEN** `alwaysSelect` is `["observability"]` and Tilt runs with no args
- **THEN** everything is enabled

#### Scenario: Invalid setting
- **WHEN** `alwaysSelect` contains an empty string
- **THEN** generation fails, naming `alwaysSelect`
