# Spec Delta

## MODIFIED Requirements

### Requirement: Tasks that cannot run under Tilt are excluded
The extension MUST NOT expose interactive tasks, tasks that run Tilt itself (command `tilt`, or the `tilt` toolchain), or tasks that depend on a task that runs Tilt, directly or through other tasks excluded for that reason, even when tagged. No resource SHALL depend on an excluded task. The generated Tiltfile SHALL name each task excluded because of such a dependency that would otherwise have been a resource, together with the dependency.

#### Scenario: Interactive task is tagged
- **WHEN** an interactive task is tagged `tilt`
- **THEN** it is not a resource

#### Scenario: Task runs Tilt
- **WHEN** a persistent task's command is `tilt up`
- **THEN** it is not a resource, and no other resource depends on it

#### Scenario: Task follows a Tilt task
- **WHEN** project `web` is tagged `tilt`, its persistent task `tilt-up` runs `tilt up`, and its persistent task `tilt-open` depends on `tilt-up`
- **THEN** neither is a resource
- **AND** the generated Tiltfile names `web:tilt-open` as excluded because it depends on `web:tilt-up`

#### Scenario: Chain of tasks after a Tilt task
- **WHEN** task `notify` depends on `tilt-open`, which depends on `tilt-up`, which runs Tilt
- **THEN** `notify` is not a resource either, and the generated Tiltfile names it as excluded because it depends on `tilt-open`

#### Scenario: Excluded task nothing would select
- **WHEN** an untagged project has a persistent task that depends on its task running `tilt up`, and `includePersistent` is off
- **THEN** the generated Tiltfile doesn't name it

#### Scenario: Unrelated tasks are unaffected
- **WHEN** a project has a task that runs Tilt, and a service that doesn't depend on it
- **THEN** the service is still a resource
