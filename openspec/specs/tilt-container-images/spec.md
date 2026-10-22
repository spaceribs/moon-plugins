# tilt-container-images Specification

## Purpose
How the Tilt extension builds container images from each project's committed, moon-generated `Dockerfile` for projects that opt in, rebuilding only what changes affect.

## Requirements

### Requirement: Images are opt-in
The extension SHALL build an image only for projects that set `project.tilt.image`. Building an image MUST NOT deploy it: the image is used by whichever of the user's Kubernetes or Compose resources reference it by name. `docker.file` settings alone MUST NOT opt a project in. With `images: false`, no images SHALL be built.

#### Scenario: Production Dockerfile settings only
- **WHEN** a project has `docker.file` settings but no `project.tilt.image`
- **THEN** no image build is generated for it

#### Scenario: Image used by the project's own manifest
- **WHEN** `api` sets `project.tilt.image: "api"` and its own manifest's container uses image `api`
- **THEN** Tilt builds the image from `api/Dockerfile` and deploys it through that manifest

#### Scenario: Image without a deploy
- **WHEN** a project sets `project.tilt.image` and no resource references the image
- **THEN** no deployment is generated for it

### Requirement: Built from the committed Dockerfile with the workspace as context
Each image SHALL be a `custom_build` named `project.tilt.image` (default: the project ID) that runs `docker build` on the project's `Dockerfile` (in the project root, where `moon docker file <project>` writes it) with the whole workspace as context. The extension MUST NOT generate Dockerfiles itself.

#### Scenario: Build command
- **WHEN** project `api` opts in
- **THEN** its build runs `docker build -t $EXPECTED_REF -f api/Dockerfile .`

### Requirement: A missing Dockerfile fails generation
When images are enabled and a project with `project.tilt.image` has no `Dockerfile` in its root, generation SHALL fail with an error naming the project and the `moon docker file <project>` command that writes one.

#### Scenario: Dockerfile not committed
- **WHEN** `api` sets `project.tilt.image` and `api/Dockerfile` doesn't exist
- **THEN** generation fails, naming `api` and `moon docker file api`

### Requirement: Rebuilds follow the project graph
An image SHALL rebuild only when the project's `Dockerfile` changes, or files change in the project's or its transitive dependencies' directories, their moon config, or files their tasks take as explicit inputs (anywhere in the workspace outside `.moon/`). Their task outputs, gitignored files, and extension config (`.moon/extensions.*`) MUST NOT trigger rebuilds.

#### Scenario: Unrelated project edited
- **WHEN** `web` is edited and `api` doesn't depend on `web`
- **THEN** `api`'s image doesn't rebuild

#### Scenario: Shared library edited
- **WHEN** a library that both `web` and `api` depend on is edited
- **THEN** both images rebuild

#### Scenario: Nested input outside the closure's projects
- **WHEN** `web` has an image, depends on `ui`, and a task of `ui` takes `/docs/ui/README.md` as an input
- **THEN** `web`'s image watches `docs/ui/README.md`, and editing it rebuilds the image

#### Scenario: Workspace-root dependency files (known limitation)
- **WHEN** `web` has an image, and the workspace root's lockfile (e.g. `pnpm-lock.yaml`), workspace or catalog manifest (e.g. `pnpm-workspace.yaml`), or root manifest (e.g. `package.json`) changes, and no task in `web`'s dependency closure takes it as an explicit input
- **THEN** `web`'s image doesn't watch it and doesn't rebuild, even though `moon docker setup` installs the image's dependencies from it; the user rebuilds with `tilt trigger` or by restarting Tilt

### Requirement: Choose which images build from source
The generated Tiltfile SHALL accept a `--to-edit` Tiltfile arg listing projects whose images build from source; absent means all, and `none` means none (published images are used). `up`, `focus`, and `ci` SHALL accept `--edit <projects>`, and `--edit-affected [--base <rev>]`, which selects the projects moon reports as affected plus their dependents, or `none` when nothing is affected.

#### Scenario: Edit affected with uncommitted changes
- **WHEN** `api` has uncommitted changes and `web` depends on `api`, and the user runs `moon ext tilt -- up --edit-affected`
- **THEN** the images of `api` and its dependents build from source, and the rest use published images

#### Scenario: Nothing affected
- **WHEN** the tree is clean and the user runs `--edit-affected`
- **THEN** Tilt runs with `--to-edit none`
