# tilt-container-images Specification

## Purpose
How the Tilt extension builds container images from moon's generated Dockerfiles for projects that opt in, rebuilding only what changes affect.

## Requirements

### Requirement: Images are opt-in
The extension SHALL build an image only for projects that set `project.tilt.k8s` (build and deploy) or `project.tilt.image` (build only). `docker.file` settings alone MUST NOT opt a project in. With `images: false`, no images SHALL be built.

#### Scenario: Production Dockerfile settings only
- **WHEN** a project has `docker.file` settings but no `project.tilt.k8s` or `project.tilt.image`
- **THEN** no image build is generated for it

### Requirement: Built from moon's Dockerfile with the workspace as context
Each image SHALL be a `custom_build` named `project.tilt.image` (default: the project ID) that runs `moon docker file <project>` into `.moon/cache/tilt/` and then `docker build` with the whole workspace as context.

#### Scenario: Build command
- **WHEN** project `api` opts in
- **THEN** its build runs `moon docker file api ... --defaults && docker build -t $EXPECTED_REF -f .moon/cache/tilt/api.Dockerfile .`

### Requirement: Rebuilds follow the project graph
An image SHALL rebuild only when files change in the project's or its transitive dependencies' directories, their moon config, or root-level files their tasks take as inputs. Their task outputs, gitignored files, and extension config (`.moon/extensions.*`) MUST NOT trigger rebuilds.

#### Scenario: Unrelated project edited
- **WHEN** `web` is edited and `api` doesn't depend on `web`
- **THEN** `api`'s image doesn't rebuild

#### Scenario: Shared library edited
- **WHEN** a library that both `web` and `api` depend on is edited
- **THEN** both images rebuild

### Requirement: Choose which images build from source
The generated Tiltfile SHALL accept a `--to-edit` Tiltfile arg listing projects whose images build from source; absent means all, and `none` means none (published images are used). `up`, `focus`, and `ci` SHALL accept `--edit <projects>`, and `--edit-affected [--base <rev>]`, which selects the projects moon reports as affected plus their dependents, or `none` when nothing is affected.

#### Scenario: Edit affected with uncommitted changes
- **WHEN** `api` has uncommitted changes and `web` depends on `api`, and the user runs `moon ext tilt -- up --edit-affected`
- **THEN** the images of `api` and its dependents build from source, and the rest use published images

#### Scenario: Nothing affected
- **WHEN** the tree is clean and the user runs `--edit-affected`
- **THEN** Tilt runs with `--to-edit none`
