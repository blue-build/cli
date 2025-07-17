# Changelog

All notable changes to this project will be documented in this file.

## [0.9.20] - 2025-07-17

### Features

- Add ability to mount secrets

### Miscellaneous Tasks

- Bump cosign to 2.5.3

## [0.9.19] - 2025-07-08

### Bug Fixes

- Only set to config/ if files/ doesn't exist

### Miscellaneous Tasks

- Release

## [0.9.18] - 2025-06-30

### Bug Fixes

- Upload-sarif comment formatting
- Allow repos that don't have a files/ directory

### Documentation

- Update README.md to bring it up to current functionality
- Add docker in list of builders

### Features

- Add the ability to set args for module calls

### Miscellaneous Tasks

- Run arm pre-build on arm runner
- Update deps
- Bump cosign
- Release

## [0.9.17] - 2025-06-02

### Bug Fixes

- Remove bootc check for now since it's causing problems

### Miscellaneous Tasks

- Release

## [0.9.16] - 2025-06-01

### Bug Fixes

- Replace / in branch names with _ when generating tags

### Miscellaneous Tasks

- Disable cache for earthly setup
- Release

## [0.9.15] - 2025-05-31

### Bug Fixes

- Remove /usr/etc in cleanup since it's not used by bootc

### Miscellaneous Tasks

- Release

## [0.9.14] - 2025-05-31

### Bug Fixes

- Needs to be bootc container lint

### Miscellaneous Tasks

- Release

## [0.9.13] - 2025-05-31

### Bug Fixes

- Setup QEMU for tag builds
- Pull akmods-extra only for bazzite (Fixes #441)
- Parse Version from container and remove ostree commit

### Miscellaneous Tasks

- Bump docker/login-action
- Add github-actions dependabot updates
- Bump cosign to 2.5.0
- Bump cosign to 2.5.0
- Fix github actions dep reference (#440)
- Use get_env_var
- Clippy fixes
- Disable legacy integration test
- Remove feature flags
- Add bootc lint
- Release

## [0.9.12] - 2025-05-09

### Bug Fixes

- Remove onig from dep tree
- Add retry for retrieving schemas
- Use our rust earthly lib now; make tests and lints more efficient
- Maximize build space for building the image
- Revert test and lint changes

### Features

- Add cache layer support

### Miscellaneous Tasks

- Clippy fixes
- Update edition to 2024
- Install toolchains and components in build
- Format files
- Rework the workflows to make it easier to manage
- Create separate test and build workflows
- Add extra test recipes
- Simplify opts using new ImageRef type
- Release

## [0.9.11] - 2025-04-15

### Bug Fixes

- Get os ID with built-ins

### Miscellaneous Tasks

- Upgrade deps
- Migrate from rinja to askama
- Upgrade cached and use new sync_writes by_key for faster operations
- Release

## [0.9.10] - 2025-03-26

### Bug Fixes

- Use sudo for skopeo copy for rechunk
- Revert change to OciDir
- Use sudo for login when using rechunk
- Fix lints and be sure to login before build in rechunk
- Handle login for skopeo during rechunk flow

### Miscellaneous Tasks

- Release

## [0.9.9] - 2025-03-23

### Bug Fixes

- Allow user to not install Nushell in their system
- Don't use * for shadow-rs build dependency

### Features

- Invoke sudo when needed for privileged

### Miscellaneous Tasks

- Add more context to schema parsing errors
- Disable logs for sensitive crates
- Bump cosign to v2.4.3
- Bump cosign image to 2.4.3
- Pin actions to commit hashes
- Upgrade deps
- Release

## [0.9.8] - 2025-02-12

### Bug Fixes

- Ignore pre-release field when parsing versions (#364)
- Filter out images whose repo or tag is <none> when listing images
- Make sure to update flake.nix during release

### Miscellaneous Tasks

- Add more context for list_images image parsing
- Release

## [0.9.7] - 2025-02-11

### Bug Fixes

- Check for buildx before using docker
- Use lenient_semver for build drivers version check to handle pre-release versions

### Miscellaneous Tasks

- Bump cosign to 2.4.2
- Release

## [0.9.6] - 2025-02-05

### Bug Fixes

- Set tags on docker build

### Miscellaneous Tasks

- Fix clippy lints
- Release

## [0.9.5] - 2025-02-01

### Bug Fixes

- Determin scripts tag

### Miscellaneous Tasks

- Release
- Release

## [0.9.4] - 2025-02-01

### Bug Fixes

- Improve validation errors

### Miscellaneous Tasks

- Use new comlexr features
- Make sure clippy checks entire workspace
- Upgrade comlexr to 1.3.0
- Cleanup code before release and update deps
- Release

## [0.9.3] - 2025-01-19

### Bug Fixes

- Don't install all features when building tag
- Remove image for docker inspect after running image to get version

### Miscellaneous Tasks

- Switch to using my new proc_macro comlexr
- Release

## [0.9.2] - 2025-01-05

### Features

- Add support for NuShell scripts
- Support versioned modules
- Add nushell completions

### Miscellaneous Tasks

- Update jsonschema
- Fix clippy lints
- Release

### Refactor

- Make use of Reference to ensure typing

## [0.9.1] - 2024-12-22

### Bug Fixes

- Prevent certain builds from running when the PR is from a fork
- Set kinoite as the default variant for generating an ISO
- Typo in --all arg for buildah and podman prune
- Use ghcr for cosign (#304)

### Features

- Add the ability to choose a tempdir for builds
- Allow fresh rechunking of image

### Miscellaneous Tasks

- Copy signing keys to `/etc/` only (#288)
- Remove unused force arg
- Use consistent syntax for getting information from os-release
- Add Github Action auditing
- Upgrade shadow-rs
- Release

### Readme

- Change file paths to match template

## [0.9.0] - 2024-12-03

### Features

- Add the ability to rechunk an image

### Miscellaneous Tasks

- Prepare for the v0.9.0 release
- Release

## [0.8.25] - 2024-12-02

### Bug Fixes

- Login to earthly for tag build-scripts-all target

### Features

- [**breaking**] Create prune command

### Miscellaneous Tasks

- Assure that `get_json_array` outputs compact `json` output
- Release

## [0.8.24] - 2024-11-27

### Bug Fixes

- Build all features for each package and build all archs
- Export get_json_array bash function
- Fix integration tests
- Add logic for inspecting multi-manifest images

### Miscellaneous Tasks

- Release

## [0.8.23] - 2024-11-26

### Bug Fixes

- Make sure tag job uses +build-images target
- Ensure we build the +build-scripts target on tags
- Make sure jq prints raw values

### Features

- Add cache for dnf5

### Miscellaneous Tasks

- Remove unneded comment about `bootupctl` command
- Add get_json_array bash function for migration to jq
- Release

## [0.8.22] - 2024-11-24

### Bug Fixes

- Update main branch workflow to use +build-images target
- Make sure to exit after unwind
- Update copy Typespec to expect proper type
- Clean up error display for validate command
- Pin prebuilds to Fedora 40
- Have integration tests job require the amd64-prebuild job
- Better support distrobox (#160)
- Setup blue-build-recipe crate to use reqwest version and features

### Features

- Add validation command
- Use yaml-rust2 to get line numbers for better errors
- Include base image information in labels
- Add the new/init subcommands (#85)

### Miscellaneous Tasks

- Cleanup workflows to be run from just (#238)
- Require integration tests to depend on prebuild
- Remove expect-exit as a dependency and add bacon config
- Remove akmod that no longer exists in integration tests
- Create dependabot.yml
- Send log files to ~/.cache/bluebuild
- Set shadow back to its original location
- Remove need to update .gitignore by making use of temporary directories
- [**breaking**] Remove force arg for build since it is no longer in use
- Update akmods image ref gen to handle open drivers
- Add extra help text for fixing local modules
- Install jq and prefer over yq for modules
- Release
- Release

## [0.8.20] - 2024-10-06

### Bug Fixes

- Ensure the correct digest is used for docker and podman inspect drivers
- Use docker buildx imagetools to inspect for the docker inspect driver
- Use full json inspection for docker inspect driver
- Switch cosign registry from GCR to GHCR (#237)
- Remove --load for docker build since we no longer pull the image for inspection

### Miscellaneous Tasks

- Fix akmods tests
- Remove akmods module for arm64 build
- Release

## [0.8.19] - 2024-10-04

### Bug Fixes

- Use built-in image inspection for podman and docker

### Miscellaneous Tasks

- Release

## [0.8.18] - 2024-10-03

### Bug Fixes

- Properly escape module json
- Add post build script to prepare image for ISO creation
- Make sigstore driver more resilient to network errors
- May not be possible to just install bootc, run bootupctl if bootc already exists
- Run image as fallback for version retrieval

### Features

- Add platform arg to force building a specific architecture

### Miscellaneous Tasks

- Add expand.rs to .gitignore for debugging macros
- Make build.rs run again on git change
- Add one more criteria for rerun build.rs to check .git/refs/heads
- Check for bootupctl in post-build script
- Remove bootupctl until issue is resolved
- Run clippy and test for every feature individually
- Release

### Refactor

- Swtich to using bon for builder pattern

## [0.8.17] - 2024-09-11

### Bug Fixes

- Fix docker login for oauth logins

### Miscellaneous Tasks

- Upgrade sigstore to use contributed changes
- Release

## [0.8.16] - 2024-09-08

### Bug Fixes

- Ensure image names are lowercase

### Miscellaneous Tasks

- Update tests for lowercasing image names
- Release

## [0.8.15] - 2024-09-07

### Bug Fixes

- Ensure that debug logs header for builds properly display the time
- Make build fail if module fails
- Generate correct image names based on user supplied arguments

### Features

- Color output in terminal if running in TTY
- Create generate-iso command (#192)
- Display list of image refs at the end of a build

### Miscellaneous Tasks

- Make sigstore an optional dep
- Update CODEOWNERS
- Update patch rev for sigstore
- Fix legacy integration tests
- Release

## [0.8.14] - 2024-08-25

### Bug Fixes

- Make sure getting version fails if not all dirs were copied
- Make sure GitHub job pushes latest image on scheduled job
- Properly handle alt-tags so they don't collide with default tags

### Miscellaneous Tasks

- Release

## [0.8.13] - 2024-08-20

### Bug Fixes

- Include $crate for macro calls
- Don't let process continue running if the main app thread panics

### Miscellaneous Tasks

- Release

### Refactor

- Create SigningDriver and CiDriver (#197)

## [0.8.12] - 2024-08-11

### Bug Fixes

- Add Ctrl-C handler for spawned children (#193)
- Support other signals properly (#194)
- Builds failing due to new Rust version
- Add typespec schemas for cli modules, remove modules.json (not needed anymore) (#209)
- Allow copying keys to both /etc and /usr/etc
- Out of bounds panic when not retrying push

### Features

- Add arm support (#191)
- Build multiple recipes in parallel (#182)
- Create RunDriver (#196)

### Miscellaneous Tasks

- Add gh cli to just release
- Build with priveleged
- Checkout proper branch and build using cargo for buildah-build
- Use proper out directory for installer image
- Capitalize AS
- Stop using secureblue for integration testing
- Move files for test-repo to work with new files module update
- Add Justfile commands for easier development (#205)
- Fix integration tests failing
- Switch from askama to rinja
- Move files from `/usr/etc/` to `/etc/` in build-time (#214)
- Release
- Fix tag CI to build prebuild separately from main build

### Refactor

- Switch to using miette for errors instead of anyhow (#198)

## [0.8.11] - 2024-06-03

### Bug Fixes

- Fail if cosign private/public key can't be verified (#190)
- Make sure username, password, and registry are not empty
- Move creds empty check to credentials module

### Documentation

- Update README to put preferred method of installation higher up

### Miscellaneous Tasks

- Add action to test external login
- Add registry for external login test
- Add external login job and buildah jobs
- Release

## [0.8.10] - 2024-05-29

### Bug Fixes

- Allow both files or config directory to not exist (#185)
- Remove extra setup call
- Remove hard requirement for login creds to be able to push (#187)

### Features

- Stages (#173)

### Miscellaneous Tasks

- Don't use satellites for integration tests
- Release

### Refactor

- [**breaking**] Rename `template` to `generate` and move `rebase/upgrade` under `switch` (#116)

## [0.8.9] - 2024-05-17

### Bug Fixes

- Don't create builder if DOCKER_HOST is set
- Use leniency for semver parsing (#184)

### Documentation

- Update README to revert cargo install instruction since issue is fixed
- Update docker/podman install instructions

### Miscellaneous Tasks

- Fix checkout for podman-build
- Remove a pre-release-replacement
- Release

## [0.8.8] - 2024-05-14

### Bug Fixes

- Add driver args to rebase/upgrade command
- Make docker pull latest images when building
- Don't use '' in format arg
- Create lock on docker setup to prevent race conditions

### Features

- Create a bluebuild buildx runner

### Miscellaneous Tasks

- Ensure cargo installs use version for build scripts image
- Cleanup install script to instead create the container without running it
- Release

## [0.8.7] - 2024-05-05

### Bug Fixes

- Git sha not present during `cargo install` (#176)

### Features

- Add alternate tags for user images (#172)

### Miscellaneous Tasks

- Streamline getting version
- Fix how we get the version in the Earthfile
- Allow tests to pass due to upstream akmods issues
- Remove title case (#177)
- Fix release replacements
- Release

## [0.8.6] - 2024-04-29

### Bug Fixes

- Fix flatpak module errors

### Miscellaneous Tasks

- Remove token from checkout
- Pull version using cargo for tag job
- Fix integration tests
- Improve tagging of images and applying labels
- Release

## [0.8.5] - 2024-04-27

### Bug Fixes

- Use shebang in release recipe
- Pull extra akmods image too (#169)

### Features

- Display full recipe with syntax highlighting (#166)
- Move module run logic into its own script (#168)

### Miscellaneous Tasks

- Fix tag.yml workflow to pull version from .workspace.package.version
- Remove debug logs from utils
- Use Semver to grab OS version from image
- Make more /var dirs
- Release

## [0.8.4] - 2024-04-22

### Bug Fixes

- Sign all images in manifest (#148)
- Use proper image URI for local rebasing
- Add test for rpm-ostree rebase (#161)
- Error if any module fails to deserialize (#163)
- Remove /var tmpfs
- Create /var/roothome to fix any issues with adding files to /root
- Create /var/lib/alternatives
- Give better errors for read_to_string

### Documentation

- Add distrobox installation tips (#146)

### Features

- Add driver selection args (#153)
- Squash builds (#155)
- Look for recipes in `./recipes/`, build files in `./files/`, and Containerfiles in `./containerfiles/` (#157)

### Miscellaneous Tasks

- Add MODULE_DIRECTORY env var (#142)
- Remove unused files module
- Put LABELS last since they cause cache miss with buildah
- Cleanup images and use hash for exports tag (#158)
- Update akmods module to account for upstream changes (#165)
- Prepare justfile for release
- Release

### README

- Add alpine distrobox and shell completions (#149)

## [0.8.3] - 2024-03-27

### Bug Fixes

- Checkout proper versions when building on main vs a PR
- Use container skopeo (#110)
- Remove tmpfs for /tmp (#123)
- Allow docker driver to properly use cache (#126)
- Allow special characters for export script (#128)
- Copy bins and keys with mounts for ostree commit (#132)
- Set gzip to default compression format
- Create dir for keys and bins in case they don't exist
- Allow user supplied registry to be set in the template (#135)
- Unable to use SHELL with podman, encapsulate commands in /bin/bash -c
- Put export script in own image
- Remove docker syntax marker
- Pulling wrong exports image

### Features

- Revert to bash files module (#125)
- Support `zstd` compression (#134)
- Improve logging output (#139)

### Miscellaneous Tasks

- Update workspace dependency versions
- Setup build concurrency to reduce number of simultaneous builds on a PR
- Adjust readme path in files module.yml
- Fix readme path for containerfile module in module.yml
- Add version checks for upstream tools (#121)
- Don't build nightly for now
- Separate nightly build to not run in CI for now
- Remove builtin-podman code
- Enable cache builds on main branch
- Don't use docker driver for buildx job on main
- Update gitlab-ci section in README
- Add image source label for exports
- Use tag exports instead
- Fix build.yml
- Release

### Refactor

- Rename strategies to drivers

## [0.8.2] - 2024-03-09

### Bug Fixes

- Filter out `/` in tag names (#94)
- Run `ostree container commit` at the end of each module run (#103)
- Add Nvidia Version to main base case (#107)
- Retry flag (#111)
- Add `org.opencontainers.image.source` LABEL for CI images (#113)
- Remove check for specific branches for signing (#114)
- Update path in comments and README (#115)

### Documentation

- Add install script from github option (#102)

### Features

- Add flakehub entry + nix flake (#109)

### Miscellaneous Tasks

- Add integration test for `disableuserns.sh` (#104)
- Update builds to use different satellites and have integration tests on their own job
- Move cargo release settings to root Cargo.toml
- Update crates to have their own versions starting at CLI version
- Prepare for v0.8.2 release

### Refactor

- Update build command to use BuildStrategy (#88)

## [0.8.1] - 2024-02-26

### Bug Fixes

- COPY yq for final image for modules to work
- COPY yq into final image for modules

### Miscellaneous Tasks

- Update modules.json to reflect change in dir layout
- Release blue-build version 0.8.1

### Refactor

- Move templates to their own crate (#83)

## [0.8.0] - 2024-02-25

### Bug Fixes

- Make sure cosign.pub exists before trying to check key validity
- Check for `GITHUB_TOKEN` instead of `SIGSTORE_ID_TOKEN` for github OIDC (#72)
- Use REGISTRY_TOKEN for GitHub OIDC signing
- Switch to using --certificate-identity-regexp for Github Keyless verification
- Remove trailing newlines from yaml arrays (#73)
- Use GH_TOKEN as GITHUB_TOKEN is a protected env var
- Allow empty custom modules dir (#77)

### Documentation

- Add module documentation for 'containerfile' and 'files' (#82)

### Features

- Use GitHub's OIDC for signing images (#62)
- Use WORKDIR and ENTRYPOINT for cli containers (#63)
- Clean up working container for SIGINT and SIGTERM (#14)
- Use tmpfs mount for /tmp and /var (#67)
- Allow user to use source images (#69)
- Make use of rpm-ostree cache (#68)
- Block overriding (#74)
- Allow use of akmods module (#71)
- Add retry options to cli build command (#81)

### Miscellaneous Tasks

- Fix build and build-pr not running properly
- Remove unwanted software so we have enough space to run the build for forked PRs
- Print out stderr from login attempts if login fails
- Replace tabs with spaces in Containerfile template
- Run integration tests on a separate satellite to keep build cache free
- Add trace log for github cosign verify
- Fix integration-tests for forks
- Update default module source (#76)
- Release blue-build version 0.8.0

### Refactor

- Use GITHUB_TOKEN instead of REGISTRY_TOKEN (#75)
- Move modules into their own directory structure (#80)

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
- Release blue-build version 0.7.1

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
