# Tasks

## 1. Verify and document

- [ ] 1.1 With real Docker (BuildKit), build a scratch workspace the way the extension does (`docker build -f <project>/Dockerfile .`) with a root `.dockerignore` and a `<project>/Dockerfile.dockerignore` that disagree. Verify the Dockerfile-specific file decides what's in the context, and that the root one applies when it's absent
- [ ] 1.2 Add the paragraph to the README's Images section, next to the root `.dockerignore` advice: the file's path, that it takes precedence, that it needs BuildKit, and that rebuild triggers come from the project graph either way. Verify it describes what 1.1 observed

## Workflow follow-up

- Archive the change (`/opsx:archive`).
- Commit as `docs(tilt): document Dockerfile-specific .dockerignore files`, closing #50.
