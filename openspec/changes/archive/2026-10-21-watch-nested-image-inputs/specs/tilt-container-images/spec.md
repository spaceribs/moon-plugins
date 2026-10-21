# Spec Delta

## MODIFIED Requirements

### Requirement: Rebuilds follow the project graph
An image SHALL rebuild only when files change in the project's or its transitive dependencies' directories, their moon config, or files their tasks take as explicit inputs (anywhere in the workspace outside `.moon/`). Their task outputs, gitignored files, and extension config (`.moon/extensions.*`) MUST NOT trigger rebuilds.

#### Scenario: Unrelated project edited
- **WHEN** `web` is edited and `api` doesn't depend on `web`
- **THEN** `api`'s image doesn't rebuild

#### Scenario: Shared library edited
- **WHEN** a library that both `web` and `api` depend on is edited
- **THEN** both images rebuild

#### Scenario: Nested input outside the closure's projects
- **WHEN** `web` has an image, depends on `ui`, and a task of `ui` takes `/docs/ui/README.md` as an input
- **THEN** `web`'s image watches `docs/ui/README.md`, and editing it rebuilds the image
