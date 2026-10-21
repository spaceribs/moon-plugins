# Spec Delta

## Purpose

Which platforms each part of the Tilt plugins supports, and the platform-specific behavior needed so the same workspace behaves the same way on each.

## ADDED Requirements

### Requirement: Core features run natively on Linux, macOS, and Windows
The Tilt extension, toolchain, and proto tool SHALL support Linux, macOS, and Windows x64 natively for everything except container images and Kubernetes deploys:
- moon tasks as Tilt resources
- project selection
- change detection
- the generated and main Tiltfiles, and templates
- workspace sync
- the toolchain's pinned Tilt

#### Scenario: Running a workspace on Windows
- **WHEN** a Windows user runs `moon ext tilt -- ci` in a workspace with tagged persistent tasks
- **THEN** Tilt loads the generated Tiltfile, and each task runs as a resource through `moon run`

#### Scenario: Selecting a project on Windows
- **WHEN** a Windows user runs `moon ext tilt -- up web`
- **THEN** Tilt enables `web`'s resources and their dependencies, as on Linux and macOS

### Requirement: Change detection behaves the same on every platform
On every supported platform, a resource SHALL re-run when a file it takes as an input changes, and SHALL NOT re-run for changes to gitignored files, its own outputs, or files that aren't its inputs.

#### Scenario: Editing an input on Windows
- **WHEN** Tilt is running on Windows and `web/schema.json`, an input of `web:codegen`, changes
- **THEN** `web:codegen` re-runs

#### Scenario: Editing a gitignored or non-input file on Windows
- **WHEN** Tilt is running on Windows and a file under gitignored `web/generated/`, or a file that no task takes as an input, changes
- **THEN** no resource re-runs

### Requirement: The pinned Tilt runs on Windows
On Windows, the extension SHALL run the toolchain's installed `tilt.exe` for the pinned version, exactly as it runs `tilt` on Linux and macOS.

#### Scenario: Toolchain on Windows
- **WHEN** the Tilt toolchain pins `0.37.8` on Windows and moon has installed it
- **THEN** `moon ext tilt -- up` runs `tilt.exe` from the toolchain's 0.37.8 install, whether or not Tilt is on `PATH`

### Requirement: Containers require Linux, macOS, or WSL2
Container images and Kubernetes or Compose workloads built by the extension SHALL be supported on Linux and macOS, which includes Windows through WSL2. On native Windows they are unsupported, and the documentation SHALL say so and point to WSL2.

#### Scenario: Images on native Windows
- **WHEN** a native Windows user reads how to build images with `project.tilt.image`
- **THEN** the documentation states that image builds need Linux, macOS, or WSL2
