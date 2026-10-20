# tilt-proto-tool Specification

## Purpose
How the Tilt proto plugin resolves Tilt versions and downloads verified prebuilt binaries for each supported platform.

## Requirements

### Requirement: Versions from git tags
The plugin SHALL list available Tilt versions from the `tilt-dev/tilt` repository's git tags, without the `v` prefix, and SHALL resolve aliases such as `latest` and partial versions.

#### Scenario: Partial version
- **WHEN** a user requests Tilt `0.33`
- **THEN** it resolves to the newest `0.33.x` release

### Requirement: Platform-specific prebuilt downloads
The plugin SHALL download `tilt.<version>.<os>.<arch>.<ext>` from the GitHub release `v<version>`, with `os` `linux`, `linux-alpine` (musl hosts), `mac`, or `windows`; `arch` `x86_64`, `arm64`, or `arm`; and `ext` `zip` on Windows or `tar.gz` elsewhere. Supported targets SHALL be Linux x64/arm64/armv7, Linux musl x64/arm64, macOS x64/arm64, and Windows x64.

#### Scenario: Alpine host
- **WHEN** installing Tilt 0.37.8 on Linux x64 with musl
- **THEN** it downloads `tilt.0.37.8.linux-alpine.x86_64.tar.gz`

#### Scenario: Unsupported target
- **WHEN** installing on Windows arm64, or Linux armv7 with musl
- **THEN** installation fails with an unsupported-target error

### Requirement: Downloads are verified
The plugin SHALL verify each download against the release's `checksums.txt`.

#### Scenario: Checksum source
- **WHEN** downloading any version
- **THEN** the checksum is read from `https://github.com/tilt-dev/tilt/releases/download/v<version>/checksums.txt`

### Requirement: Executable
The plugin SHALL expose `tilt` (`tilt.exe` on Windows) as the primary executable.

#### Scenario: Shim
- **WHEN** Tilt is installed with proto
- **THEN** `tilt` is available through proto's shims
