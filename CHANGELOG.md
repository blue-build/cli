# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.3.7 (2023-12-30)

### Bug Fixes

 - <csr-id-80477025825f9b8c01eea768950fc263876e64dd/> update README to point to new project

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update README to point to new project (8047702)
</details>

## v0.3.6 (2023-12-30)

### New Features

 - <csr-id-6a15c56a906c51332aa5e5766c928692004ce97c/> Add Github support in Build command

### Bug Fixes

 - <csr-id-88cc37529a2c8b8d43e8610ea282c5c9951fa721/> bump version
 - <csr-id-7bae446d2782de142a38d45c60798860da76cc7a/> Update cargo.toml
 - <csr-id-75dc31182f4cf9aa9ee1d034fdf548163513a563/> logging

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 1 day passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.6 (ff3f066)
    - Bump version (88cc375)
    - Release ublue-rs v0.3.5 (9387d20)
    - Update cargo.toml (7bae446)
    - Fix changelog (bfdd3ce)
    - Logging (75dc311)
    - Add Github support in Build command (6a15c56)
</details>

## v0.3.5 (2023-12-28)

<csr-id-aa86f48a5d21c6d3358f1426486ca46d7ea98e72/>

### Chore

 - <csr-id-aa86f48a5d21c6d3358f1426486ca46d7ea98e72/> Switch to using typed builders

### Documentation

 - <csr-id-c559fb4d6b729b9e83ff9cfe94ded24b52949f94/> Update README and CHANGELOG

### New Features

 - <csr-id-7ce70480bfc7e3bb669db0224cfa91bedf2bed6a/> Add basic templating support for Github Actions
 - <csr-id-6c61cab07e1fce2373053b2f04034f4b2761284f/> Add main README template
 - <csr-id-249f852a3faf8fcbde03bbc2cf7d9240a72940e3/> Adding new subcommand
 - <csr-id-556652f92a805209585aa8a76dbba98a12fdc5b6/> Adding more template files for init
 - <csr-id-6a15c56a906c51332aa5e5766c928692004ce97c/> Add Github support in Build command

### Bug Fixes

 - <csr-id-3b07758709e809f73749f652797ca382cfe526ac/> add support for alpine image and using either podman or buildah
 - <csr-id-7bae446d2782de142a38d45c60798860da76cc7a/> Update cargo.toml
 - <csr-id-75dc31182f4cf9aa9ee1d034fdf548163513a563/> logging

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.5 (386308f)
    - Update changelog for release (7897f9d)
    - Add basic templating support for Github Actions (7ce7048)
    - Add support for alpine image and using either podman or buildah (3b07758)
    - Add main README template (6c61cab)
    - Adding new subcommand (249f852)
    - Adding more template files for init (556652f)
    - Update README and CHANGELOG (c559fb4)
    - Add ability to use incremental caching for rust builds in Earthfile (a25e041)
    - Have ublue-cli manage iso-generator (a3da7e3)
    - Switch to using typed builders (aa86f48)
    - Fix SAVE IMAGE (e9cfc8a)
</details>

## v0.3.4 (2023-12-19)

<csr-id-603a3335f9cd97a1905dae6c909f95bcff051686/>
<csr-id-ea6143c7f7e628ea2958ccf8193c4e0e68595d2c/>
<csr-id-13d10bedf951d70a1f42c6dbebd0098ec7a2a610/>
<csr-id-1f030d6b74ebfb2f47b6772da59529be0996a2de/>
<csr-id-5b1f99759c77377c8fbbe9c79b87d5c09c6479cd/>

### Chore

 - <csr-id-603a3335f9cd97a1905dae6c909f95bcff051686/> Fix version to match with published version

### Chore

 - <csr-id-5b1f99759c77377c8fbbe9c79b87d5c09c6479cd/> add rusty-hook

### New Features (BREAKING)

 - <csr-id-785fc2f7621782a1597b728c5b5e49c41364e85e/> Remove legacy code"

### Bug Fixes

 - <csr-id-006966bb351a34c435726dd7e202790001005a7c/> Move command structs into bin
 - <csr-id-49d512b3f18db3370554c0e5b57b2dfc5d1d2abd/> Make containerfile formatting nicer
 - <csr-id-b374d546833eee10ccf9f7c8203fd5fe6637e143/> Remove single quotes from image_digest
 - <csr-id-f437bdaffa48d45e510a34cbd09d0ff736f92ded/> clippy
 - <csr-id-7f4f666b0aa82c6fe04c6c3a96630a59b7c1a675/> improper trim of image digest

### New Features

 - <csr-id-4ea6f772e083419f74a5a304674b58c54a4c6a99/> Finish build feature
 - <csr-id-b83cf574b8417a482c7f7fbfe9d3699aa9bd1d50/> Add logging

### Chore

 - <csr-id-ea6143c7f7e628ea2958ccf8193c4e0e68595d2c/> fix .git/ exclude
 - <csr-id-13d10bedf951d70a1f42c6dbebd0098ec7a2a610/> Exclude some more files
 - <csr-id-1f030d6b74ebfb2f47b6772da59529be0996a2de/> Add rust-toolchain.toml

### Documentation

 - <csr-id-33bee78acfd25b89599b5e26646c9faeaf1576f8/> Update README, checking off a feature

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.4 (f47dd02)
    - Update changelog for release (785a60f)
    - Refactor Command Structs and create Earthly build (ebd861c)
</details>

## 0.3.3 (2023-12-18)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.3 (c20b917)
    - Update changelog for release (a5663af)
    - Set some env vars for cosign; force color logs (d936000)
</details>

## 0.3.2 (2023-12-18)

### Bug Fixes

 - <csr-id-7f4f666b0aa82c6fe04c6c3a96630a59b7c1a675/> improper trim of image digest

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.2 (f8214fc)
    - Update changelog for release (211a393)
    - Improper trim of image digest (7f4f666)
</details>

## 0.3.1 (2023-12-18)

<csr-id-5b1f99759c77377c8fbbe9c79b87d5c09c6479cd/>

### Chore

 - <csr-id-5b1f99759c77377c8fbbe9c79b87d5c09c6479cd/> add rusty-hook

### New Features

 - <csr-id-b83cf574b8417a482c7f7fbfe9d3699aa9bd1d50/> Add logging

### Bug Fixes

 - <csr-id-b374d546833eee10ccf9f7c8203fd5fe6637e143/> Remove single quotes from image_digest
 - <csr-id-f437bdaffa48d45e510a34cbd09d0ff736f92ded/> clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.1 (8878751)
    - Update changelog for release (73b9a1d)
    - Remove single quotes from image_digest (b374d54)
    - Add logging (b83cf57)
    - Clippy (f437bda)
    - Add rusty-hook (5b1f997)
</details>

## 0.3.0 (2023-12-17)

<csr-id-ea6143c7f7e628ea2958ccf8193c4e0e68595d2c/>
<csr-id-13d10bedf951d70a1f42c6dbebd0098ec7a2a610/>
<csr-id-1f030d6b74ebfb2f47b6772da59529be0996a2de/>

### Chore

 - <csr-id-ea6143c7f7e628ea2958ccf8193c4e0e68595d2c/> fix .git/ exclude
 - <csr-id-13d10bedf951d70a1f42c6dbebd0098ec7a2a610/> Exclude some more files
 - <csr-id-1f030d6b74ebfb2f47b6772da59529be0996a2de/> Add rust-toolchain.toml

### New Features

 - <csr-id-4ea6f772e083419f74a5a304674b58c54a4c6a99/> Finish build feature

### Bug Fixes

 - <csr-id-006966bb351a34c435726dd7e202790001005a7c/> Move command structs into bin
 - <csr-id-49d512b3f18db3370554c0e5b57b2dfc5d1d2abd/> Make containerfile formatting nicer

### New Features (BREAKING)

 - <csr-id-785fc2f7621782a1597b728c5b5e49c41364e85e/> Remove legacy code"

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 42 calendar days.
 - 43 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.0 (7745830)
    - Update changelog for release (985a3f6)
    - Finish build feature (4ea6f77)
    - Start work on build command (71d9397)
    - Update README (bcd7e71)
    - Remove legacy code" (785fc2f)
    - Move command structs into bin (006966b)
    - Make containerfile formatting nicer (49d512b)
    - Fix .git/ exclude (ea6143c)
    - Exclude some more files (13d10be)
    - Add rust-toolchain.toml (1f030d6)
</details>

## 0.2.2 (2023-11-04)

<csr-id-603a3335f9cd97a1905dae6c909f95bcff051686/>

### Chore

 - <csr-id-603a3335f9cd97a1905dae6c909f95bcff051686/> Fix version to match with published version

### Documentation

 - <csr-id-33bee78acfd25b89599b5e26646c9faeaf1576f8/> Update README, checking off a feature

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 3 calendar days.
 - 7 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.2.2 (df7cc56)
    - Update changelog for release (a811667)
    - Fix version to match with published version (603a333)
    - Update README, checking off a feature (33bee78)
    - Comment out config for now (5968065)
</details>

## 0.2.0 (2023-10-28)

### Bug Fixes

 - <csr-id-c6f2e5b18de8a85e482583fed075fb25818d7f34/> print module context as json
 - <csr-id-52936fffb195b837c9e93be4a99f9964fadae1e4/> remove tracing
 - <csr-id-f2ab9bfd4aeb98709a8fc8aaed7b535c3010a4ad/> Encapsulate module echo in quotes to be passed in as a single arg
 - <csr-id-b1b2b0b2ac2be3655066317246486bc337f38ad4/> Add support for legacy containerfiles and modules containerfiles
 - <csr-id-731e1d75671ebc764dbfa052c72b0df3fbd1141a/> Create README

### New Features (BREAKING)

 - <csr-id-9564ca0af36cb9ba26ba6c158dd1e24db90deee5/> Allow containerfile module to print out to main Containerfile
 - <csr-id-85aadf73e520b5f0c14c2e98093745c45a52b0c1/> Support new modules based starting point template

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 12 calendar days.
 - 12 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.2.0 (6b0f684)
    - Print module context as json (c6f2e5b)
    - Allow containerfile module to print out to main Containerfile (9564ca0)
    - Remove tracing (52936ff)
    - Encapsulate module echo in quotes to be passed in as a single arg (f2ab9bf)
    - Add support for legacy containerfiles and modules containerfiles (b1b2b0b)
    - Support new modules based starting point template (85aadf7)
    - Create README (731e1d7)
</details>

## 0.1.1 (2023-10-16)

### Bug Fixes

 - <csr-id-b39fb4cf1f19fee8ddd183a2401fe88143cf4dd7/> add changelog

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 27 commits contributed to the release over the course of 20 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.1.1 (0e4036b)
    - Release ublue-rs v0.1.1 (5d3756b)
    - Add changelog (b39fb4c)
    - Revert back to published version number (acc29d6)
    - Ignore the .sccache dir just in case (c6a339c)
    - Remove license-file prop (89acdbc)
    - Include the cargo build pipeline (7f89c4e)
    - Add features section (5c503ef)
    - Put init and build behind feature flags (073ad4c)
    - Make changes to exclude and license (90bab6c)
    - Start work on pipeline (21beccd)
    - Start work on init command (564ea91)
    - Don't specify specific include (bc4557a)
    - Set bin (6b6578b)
    - Remove default-run (fcf653d)
    - Update Cargo.toml in order to publish (6753ab5)
    - Fix recipe and templates (8668a7a)
    - Fix template (c415f6a)
    - Clean up the code a bit (bd04489)
    - Create autorun script capabilities (2cd8878)
    - Add LICENSE (a28f0af)
    - Allow for custom Containerfile adding (69effba)
    - Get cli in basic working order (bd6fabd)
    - Able to generate a Containerfile (e42cda0)
    - Making some progress (5361b36)
    - Create templates, serialization structs, and cli arg parsing (783c53e)
    - Initial commit (6a7cadd)
</details>

