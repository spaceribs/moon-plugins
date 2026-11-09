# Spec Delta

## ADDED Requirements

### Requirement: Build options
A project's `project.tilt.imageBuild` SHALL add options to its image's `docker build`: each `ssh` entry as `--ssh <value>`, each `args` entry as `--build-arg <NAME>=<value>`, `target` as `--target <value>`, and each `secrets` entry as `--secret <value>`, quoted for the shell where needed. Without the setting, the command SHALL be unchanged.

#### Scenario: SSH agent for private dependencies
- **WHEN** project `api` sets `imageBuild: { ssh: ["default"] }`
- **THEN** its build runs `docker build -t $EXPECTED_REF -f api/Dockerfile --ssh default .`

#### Scenario: Every option
- **WHEN** `api` sets `imageBuild: { target: "dev", args: { GOPRIVATE: "github.com/acme/*" }, ssh: ["default"], secrets: ["id=npmrc,src=.npmrc"] }`
- **THEN** its build runs `docker build -t $EXPECTED_REF -f api/Dockerfile --target dev --build-arg 'GOPRIVATE=github.com/acme/*' --ssh default --secret id=npmrc,src=.npmrc .`

### Requirement: Invalid build options fail generation
Generation SHALL fail, naming the project and the option, when `project.tilt.imageBuild` isn't an object, has a key other than `ssh`, `args`, `target`, or `secrets`, `ssh` or `secrets` isn't a list of non-empty strings, `args` isn't an object of strings whose names are non-empty and contain no `=` or whitespace, `target` isn't a non-empty string, or the project doesn't set `project.tilt.image`.

#### Scenario: Unknown option
- **WHEN** `api` sets `imageBuild: { platform: "linux/amd64" }`
- **THEN** generation fails, naming `api` and `platform`

#### Scenario: Build options without an image
- **WHEN** `web` sets `imageBuild: { ssh: ["default"] }` but not `project.tilt.image`
- **THEN** generation fails, naming `web` and `imageBuild`
