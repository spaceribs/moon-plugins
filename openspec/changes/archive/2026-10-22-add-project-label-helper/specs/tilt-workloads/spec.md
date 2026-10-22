# Spec Delta

## ADDED Requirements

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
