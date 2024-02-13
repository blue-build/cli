# Changelog

All notable changes to this project will be documented in this file.

## [0.7.1] - 2024-02-13

### Bug Fixes

- Remove deprecated bling `COPY` for `files` and `rpms` (#52)
- Only use earthly builder if token exists (#53)

### Features

- Use Multi-stage builds to prevent COPY for modules and config (#54)
- Alias update for upgrade subcommand (#60)

### Miscellaneous Tasks

- Update /Containerfile in .gitignore
- Create base integration test setup (#55)
- Remove nightly flags
- Rename registry-path arg to registry-namespace but keep previous as alias
- Add cargo release files

### Refactor

- Enable clippy nursery lint

## [0.7.0] - 2024-02-07

### Features

- Snippets (#51)

### Refactor

- [**breaking**] Rename bb to bluebuild (#50)

## [0.6.0] - 2024-02-06

### Bug Fixes

- Tag workflow version fix (#16)
- Improper syntax for test in tag workflow
- Improve workflow for main branch and PRs (#17)
- Use new cargo-builder to help speed up build times
- Change local build dir to /etc/bluebuild
- Build failing due to change in local tarball location
- Add missing container tags (#37)
- Update containerfile to check for presence of cosign.pub (#46)
- Output better serde::yaml errors (#47)
- Lowecase registry and update IMAGE_REGISTRY arg (#49)

### Features

- Add release workflows (#22)
- Upgrades (#26)
- Bugreport command (#28)
- Use COPY syntax for files module (#38)
- Allow default recipe path (#45)

### Miscellaneous Tasks

- Move recipe out to its own module (#18)
- Enable Clippy Pedantic lint (#19)
- Fix simple error in workflow (#27)
- Update/Remove logos in this repo (#23) (#30)
- Setup earthly satellite building (#29)
- Update README to show github action use
- Set version to 0.5.6-dev.0 to prepare for first release
- Switch back to crate format_serde_error
- Prepare for 0.6.0 release

### Refactor

- Separate module template from recipe module (#32)
- Separate modules into individual templates

## [0.5.5] - 2024-01-26

### Bug Fixes

- Install script not working as intended (#15)

### Documentation

- Update gitlab ci example
- Update README for distrobox usage (#12)

### Miscellaneous Tasks

- Bumb version

## [0.5.4] - 2024-01-24

### Miscellaneous Tasks

- Don't fetch tags again
- Add token for pushing tags
- Bump version
- Bump version

## [0.5.3] - 2024-01-24

### Miscellaneous Tasks

- Bump version

## [0.5.2] - 2024-01-24

### Bug Fixes

- Update outdated 60-custom.just
- Rebase path not being generated properly (#8)

### Documentation

- Update changelog
- Manual update changelog for release

### Features

- Run clippy + BlueBuildTrait (#4)

### Miscellaneous Tasks

- Update Cargo.toml with new repo URL
- Manual bump of version
- Create GitHub Workflow (#9)
- Don't build integration tests in +all
- Allow write for contents and id-token
- Allow workflow_dispatch on build
- Use docker/login-action@v3
- Set packages permissions to write
- Update README.md (#10)
- Use GHCR for install.sh (#11)
- Remove input for release
- Add CARGO_REGISTRY_TOKEN
- Fetch all to get history for changelog updates
- Allow write for id-token

## [0.5.1] - 2024-01-22

### Bug Fixes

- Allow single module from-file

### Documentation

- Update README for upgrade and rebase commands

## [0.5.0] - 2024-01-21

### Features

- [**breaking**] Upgrade and Rebase commands

## [0.4.3] - 2024-01-19

### Miscellaneous Tasks

- Add CODEOWNERS file
- Enable integration tests
- Run both nightly and default integration tests
- Use --privileged instead of WITH DOCKER

### Testing

- Add integration tests for build and template

### Nightly

- Use podman-api crate for building images

## [0.4.2] - 2024-01-14

### Bug Fixes

- Used wrong image for installer in Containerfile template

## [0.4.1] - 2024-01-14

### Bug Fixes

- Installer used wrong image tag

### Documentation

- Update README to describe using local builds

## [0.4.0] - 2024-01-14

### Features

- [**breaking**] Remove containerfile arg since we use compiled time templates

## [0.3.13] - 2024-01-14

### Bug Fixes

- Conflicting short args for build subcommand

### Features

- Local image rebasing

## [0.3.12] - 2024-01-06

### Documentation

- Add logos

## [0.3.11] - 2024-01-04

### Bug Fixes

- Removed unwrap from template to handle with proper error message

## [0.3.10] - 2024-01-04

### Bug Fixes

- Stop possible from-file, type module collision in template

### Refactor

- Use askama crate for compile-time template type checking

## [0.3.9] - 2024-01-01

### Bug Fixes

- Earthfile syntax error
- Allow image_version to be a String
- Clippy error for image_tag

### Refactor

- Inefficiency in generated Containerfile

## [0.3.8] - 2023-12-30

### Bug Fixes

- Rename ublue-rs to blue-build

### Documentation

- Renaming tool in docs

## [0.3.7] - 2023-12-30

### Bug Fixes

- Update README to point to new project

## [0.3.6] - 2023-12-30

### Bug Fixes

- Logging
- Update cargo.toml
- Bump version

### Features

- Add Github support in Build command

## [0.3.5] - 2023-12-28

### Bug Fixes

- Add support for alpine image and using either podman or buildah

### Documentation

- Update README and CHANGELOG

### Features

- Adding more template files for init
- Adding new subcommand
- Add main README template
- Add basic templating support for Github Actions

### Miscellaneous Tasks

- Switch to using typed builders

## [0.3.2] - 2023-12-18

### Bug Fixes

- Improper trim of image digest

## [0.3.1] - 2023-12-18

### Bug Fixes

- Clippy
- Remove single quotes from image_digest

### Features

- Add logging

### Miscellaneous Tasks

- Add rusty-hook

## [0.3.0] - 2023-12-17

### Bug Fixes

- Make containerfile formatting nicer
- Move command structs into bin

### Features

- [**breaking**] Remove legacy code"
- Finish build feature

### Miscellaneous Tasks

- Add rust-toolchain.toml
- Exclude some more files
- Fix .git/ exclude

## [0.2.2] - 2023-11-04

### Documentation

- Update README, checking off a feature

### Miscellaneous Tasks

- Fix version to match with published version

## [0.2.0] - 2023-10-28

### Bug Fixes

- Create README
- Add support for legacy containerfiles and modules containerfiles
- Encapsulate module echo in quotes to be passed in as a single arg
- Remove tracing
- Print module context as json

### Features

- [**breaking**] Support new modules based starting point template
- [**breaking**] Allow containerfile module to print out to main Containerfile

## [0.1.1] - 2023-10-16

### Bug Fixes

- Add changelog

<!-- generated by git-cliff -->
