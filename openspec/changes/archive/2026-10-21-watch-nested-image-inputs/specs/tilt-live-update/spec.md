# Spec Delta

## MODIFIED Requirements

### Requirement: Some files need a full rebuild
Changes to moon config, root-level task inputs (e.g. lockfiles), and explicit task input files (e.g. `package.json`) SHALL trigger a full image rebuild instead of a sync. Every file that triggers a full rebuild SHALL also be watched by the image, so the generated Tiltfile always loads.

#### Scenario: moon.yml edited
- **WHEN** `api/moon.yml` changes
- **THEN** the image is rebuilt and redeployed

#### Scenario: Nested explicit input
- **WHEN** a task in the image's dependency closure takes `/docs/ui/README.md` as an input
- **THEN** `docs/ui/README.md` is both a full-rebuild file and watched by the image, and Tilt loads the Tiltfile
