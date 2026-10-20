# Spec Delta

## MODIFIED Requirements

### Requirement: Images are opt-in
The extension SHALL build an image only for projects that set `project.tilt.image`. Building an image MUST NOT deploy it: the image is used by whichever of the user's Kubernetes or Compose resources reference it by name. `docker.file` settings alone MUST NOT opt a project in. With `images: false`, no images SHALL be built.

#### Scenario: Production Dockerfile settings only
- **WHEN** a project has `docker.file` settings but no `project.tilt.image`
- **THEN** no image build is generated for it

#### Scenario: Image used by the project's own manifest
- **WHEN** `api` sets `project.tilt.image: "api"` and its own manifest's container uses image `api`
- **THEN** Tilt builds the image from moon's Dockerfile and deploys it through that manifest

#### Scenario: Image without a deploy
- **WHEN** a project sets `project.tilt.image` and no resource references the image
- **THEN** no deployment is generated for it
