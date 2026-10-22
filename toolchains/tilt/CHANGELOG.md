# Changelog

## [0.1.2](https://github.com/spaceribs/moon-plugins/compare/tilt_toolchain-v0.1.1...tilt_toolchain-v0.1.2) - 2026-10-22

### Fixed

- *(tilt)* skip generation during workspace sync inside images

### Other

- *(tilt)* MOON_SKIP_SYNC_WORKSPACE doesn't avoid loading the extension
- tighten the READMEs and fix inaccuracies

## [0.1.1](https://github.com/spaceribs/moon-plugins/compare/tilt_toolchain-v0.1.0...tilt_toolchain-v0.1.1) - 2026-10-21

### Fixed

- *(tilt)* run the toolchain's installed Tilt instead of relying on PATH and proto shims

### Other

- cache dependency builds and moon's task cache, and lint on Linux only
- add release-plz and commit linting

## [0.1.0] - 2026-10-20

Initial release.

#### 🚀 Features

- Install and pin Tilt through proto (bundles the Tilt proto plugin), on `PATH` for tasks that run `tilt` and for the Tilt extension.
