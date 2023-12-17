# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

<csr-id-603a3335f9cd97a1905dae6c909f95bcff051686/>

### Chore

 - <csr-id-603a3335f9cd97a1905dae6c909f95bcff051686/> Fix version to match with published version

### New Features (BREAKING)

 - <csr-id-785fc2f7621782a1597b728c5b5e49c41364e85e/> Remove legacy code"

### Bug Fixes

 - <csr-id-006966bb351a34c435726dd7e202790001005a7c/> Move command structs into bin
 - <csr-id-49d512b3f18db3370554c0e5b57b2dfc5d1d2abd/> Make containerfile formatting nicer

### New Features

 - <csr-id-4ea6f772e083419f74a5a304674b58c54a4c6a99/> Finish build feature

### Chore

 - <csr-id-ea6143c7f7e628ea2958ccf8193c4e0e68595d2c/> fix .git/ exclude
 - <csr-id-13d10bedf951d70a1f42c6dbebd0098ec7a2a610/> Exclude some more files
 - <csr-id-1f030d6b74ebfb2f47b6772da59529be0996a2de/> Add rust-toolchain.toml

### Documentation

 - <csr-id-33bee78acfd25b89599b5e26646c9faeaf1576f8/> Update README, checking off a feature

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 42 calendar days.
 - 43 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
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

