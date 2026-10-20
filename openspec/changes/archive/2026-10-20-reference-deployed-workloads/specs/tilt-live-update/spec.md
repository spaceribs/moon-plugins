# Spec Delta

## MODIFIED Requirements

### Requirement: Restart apps without live reload
When the project's `serviceRestart` is `inputs` and it has a `docker.file.startTask`, the app SHALL be restarted after syncing, re-running `moon run <project>:<startTask>`: through Tilt's `restart_process` extension, or, when every workload the project declares is a Compose service, by restarting the container with `restart_container()`, since Tilt doesn't support `restart_process`'s entrypoint override for Compose. Otherwise the app SHALL be left to reload itself.

#### Scenario: Plain node server
- **WHEN** `api` sets `serviceRestart: "inputs"` and its source is synced
- **THEN** its process restarts with the new code

#### Scenario: Compose-deployed server
- **WHEN** `api` sets `serviceRestart: "inputs"` and `workloads: { compose: ["api"] }`, and its source is synced
- **THEN** the file is synced and the `api` container restarts, re-running `moon run api:serve`, without an image rebuild
