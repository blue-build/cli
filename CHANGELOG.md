# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.5.4 (2024-01-24)

### Chore

 - <csr-id-f27475ca39bd341f267cee5b7da65dbb93ec77ff/> Bump version
 - <csr-id-c59caf45fb3866c2c97b10e1c6972e1447d80aac/> Bump version

### Other

 - <csr-id-8c9a5fc5ec8d401c18064605fbd8fea57c2bc616/> Add token for pushing tags
 - <csr-id-4d2e56292d3f54df6b7df20c5337fa8fb0e6fc63/> Don't fetch tags again

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump version ([`f27475c`](https://github.com/blue-build/cli/commit/f27475ca39bd341f267cee5b7da65dbb93ec77ff))
    - Bump version ([`c59caf4`](https://github.com/blue-build/cli/commit/c59caf45fb3866c2c97b10e1c6972e1447d80aac))
    - Add token for pushing tags ([`8c9a5fc`](https://github.com/blue-build/cli/commit/8c9a5fc5ec8d401c18064605fbd8fea57c2bc616))
    - Don't fetch tags again ([`4d2e562`](https://github.com/blue-build/cli/commit/4d2e56292d3f54df6b7df20c5337fa8fb0e6fc63))
</details>

## v0.5.3 (2024-01-24)

<csr-id-56ab314a44a9d3a8d35bb6173646746054368bf6/>

### Chore

 - <csr-id-56ab314a44a9d3a8d35bb6173646746054368bf6/> Bump version

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump blue-build v0.5.3 ([`b6a50e2`](https://github.com/blue-build/cli/commit/b6a50e213862ef929aeff5caf89bc616b4696ecb))
    - Bump version ([`56ab314`](https://github.com/blue-build/cli/commit/56ab314a44a9d3a8d35bb6173646746054368bf6))
    - Revert "Bump blue-build v0.5.2" ([`aee4182`](https://github.com/blue-build/cli/commit/aee4182ea2074e08a9a69d6e8044eaf970874370))
    - Reapply "ci: Fetch all to get history for changelog updates" ([`a289739`](https://github.com/blue-build/cli/commit/a289739dc4ac951c6c95a4f5038ed21c6a5f2cef))
    - Revert "ci: Fetch all to get history for changelog updates" ([`9faf35e`](https://github.com/blue-build/cli/commit/9faf35e8a983f3dff1d99cacf2894edf948a0b85))
</details>

## v0.5.2 (2024-01-24)

<csr-id-4f62b3e5df47b74191a4222a6d98c9ffc5d7a993/>
<csr-id-4858d9dc3022edfdd7a0f7b31f42aa503c75fe9b/>
<csr-id-8bc7cf3a0a7cf4d3995bdcb60071aa2912801ae5/>
<csr-id-32d31fdf6c7bfc3d577e6ac1d36bf504624887b1/>
<csr-id-cf04653458059560b8f735e795b72ffd4b1a418c/>
<csr-id-40d6ffbddef078b7c37e363a8f64ad70e13a5ca9/>
<csr-id-90dbe0bdedc291459427cc3ad22d6296d79e2b05/>
<csr-id-5849c4a23febb8f4a5435824cb72e9cee9aba8bf/>
<csr-id-e8e8bfa0966cd6b2bf827620aad6df7fb5339b36/>
<csr-id-6a4c89d567c4f7cd76415c56818527d9f7048bc2/>
<csr-id-99649d2d8804c052ee41e51dbce822ff3ef2fe74/>
<csr-id-42d879a6e52cc466a1e12d754eaa26ce7b21015a/>
<csr-id-00b81a25bc546eb7314e4b910f2b4c101b7ee9fc/>
<csr-id-dbea80c94586b1d34e372639d01f4cf2baa0f2e7/>

### Chore

 - <csr-id-4f62b3e5df47b74191a4222a6d98c9ffc5d7a993/> Update Cargo.toml with new repo URL

### Other

 - <csr-id-4858d9dc3022edfdd7a0f7b31f42aa503c75fe9b/> Allow write for id-token
 - <csr-id-8bc7cf3a0a7cf4d3995bdcb60071aa2912801ae5/> Fetch all to get history for changelog updates
 - <csr-id-32d31fdf6c7bfc3d577e6ac1d36bf504624887b1/> Add CARGO_REGISTRY_TOKEN
 - <csr-id-cf04653458059560b8f735e795b72ffd4b1a418c/> Remove input for release
 - <csr-id-40d6ffbddef078b7c37e363a8f64ad70e13a5ca9/> Set packages permissions to write
 - <csr-id-90dbe0bdedc291459427cc3ad22d6296d79e2b05/> Use docker/login-action@v3
 - <csr-id-5849c4a23febb8f4a5435824cb72e9cee9aba8bf/> Allow workflow_dispatch on build
 - <csr-id-e8e8bfa0966cd6b2bf827620aad6df7fb5339b36/> Allow write for contents and id-token
 - <csr-id-6a4c89d567c4f7cd76415c56818527d9f7048bc2/> Don't build integration tests in +all
 - <csr-id-99649d2d8804c052ee41e51dbce822ff3ef2fe74/> Create GitHub Workflow
   Add support for building, tagging, and releasing via CICD
   
   ---------

### Documentation

 - <csr-id-d005bfc9251313e96687754634bb42178e642a48/> Manual update changelog for release
 - <csr-id-e71b1897d1e752008a407d27b40fd31c7465e6be/> Update changelog

### Chore

 - <csr-id-42d879a6e52cc466a1e12d754eaa26ce7b21015a/> use GHCR for install.sh
   Trying to make the action install the binary from the container on GHCR
   with `podman run --rm ghcr.io/blue-build/cli:main-installer | sudo bash`
   (like in the README). Getting some error and this _might_ be related and
   might not, but shouldn't hurt to merge either, since I just made the
   `cli` package public.
   
   ---------
 - <csr-id-00b81a25bc546eb7314e4b910f2b4c101b7ee9fc/> Update README.md
   update description
 - <csr-id-dbea80c94586b1d34e372639d01f4cf2baa0f2e7/> Manual bump of version

### New Features

 - <csr-id-dbbd087b5ba3dc7a6bf8f3fa9be2cdb569d6cecb/> run clippy + BlueBuildTrait
   * feat: run clippy + BlueBuildTrait
* chore: add default run impl; more clippy
* chore: remove vscode folder; not needed
* cleanups
* Move to commands.rs
* Move functions; remove run function implementation from each command
* Remove run impl from init commands
* Use error log

### Bug Fixes

<csr-id-9454baa75063b7c249fde08f663b5982c0572384/>

 - <csr-id-c832bcd1aaca3b29ee45accf1673610aaffaf888/> Rebase path not being generated properly
   * fix: Rebase path not being generated properly
* consolidate logic into generate_full_image_name
* Fix nightly build

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release over the course of 3 calendar days.
 - 3 days passed between releases.
 - 19 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#10](https://github.com/blue-build/cli/issues/10), [#11](https://github.com/blue-build/cli/issues/11), [#4](https://github.com/blue-build/cli/issues/4), [#8](https://github.com/blue-build/cli/issues/8), [#9](https://github.com/blue-build/cli/issues/9)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#10](https://github.com/blue-build/cli/issues/10)**
    - Update README.md ([`00b81a2`](https://github.com/blue-build/cli/commit/00b81a25bc546eb7314e4b910f2b4c101b7ee9fc))
 * **[#11](https://github.com/blue-build/cli/issues/11)**
    - Use GHCR for install.sh ([`42d879a`](https://github.com/blue-build/cli/commit/42d879a6e52cc466a1e12d754eaa26ce7b21015a))
 * **[#4](https://github.com/blue-build/cli/issues/4)**
    - Run clippy + BlueBuildTrait ([`dbbd087`](https://github.com/blue-build/cli/commit/dbbd087b5ba3dc7a6bf8f3fa9be2cdb569d6cecb))
 * **[#8](https://github.com/blue-build/cli/issues/8)**
    - Rebase path not being generated properly ([`c832bcd`](https://github.com/blue-build/cli/commit/c832bcd1aaca3b29ee45accf1673610aaffaf888))
 * **[#9](https://github.com/blue-build/cli/issues/9)**
    - Create GitHub Workflow ([`99649d2`](https://github.com/blue-build/cli/commit/99649d2d8804c052ee41e51dbce822ff3ef2fe74))
 * **Uncategorized**
    - Bump blue-build v0.5.2 ([`c3003e2`](https://github.com/blue-build/cli/commit/c3003e27c50e06e60e18b6e5180fe0c077c8975c))
    - Allow write for id-token ([`4858d9d`](https://github.com/blue-build/cli/commit/4858d9dc3022edfdd7a0f7b31f42aa503c75fe9b))
    - Fetch all to get history for changelog updates ([`8bc7cf3`](https://github.com/blue-build/cli/commit/8bc7cf3a0a7cf4d3995bdcb60071aa2912801ae5))
    - Add CARGO_REGISTRY_TOKEN ([`32d31fd`](https://github.com/blue-build/cli/commit/32d31fdf6c7bfc3d577e6ac1d36bf504624887b1))
    - Remove input for release ([`cf04653`](https://github.com/blue-build/cli/commit/cf04653458059560b8f735e795b72ffd4b1a418c))
    - Set packages permissions to write ([`40d6ffb`](https://github.com/blue-build/cli/commit/40d6ffbddef078b7c37e363a8f64ad70e13a5ca9))
    - Use docker/login-action@v3 ([`90dbe0b`](https://github.com/blue-build/cli/commit/90dbe0bdedc291459427cc3ad22d6296d79e2b05))
    - Allow workflow_dispatch on build ([`5849c4a`](https://github.com/blue-build/cli/commit/5849c4a23febb8f4a5435824cb72e9cee9aba8bf))
    - Allow write for contents and id-token ([`e8e8bfa`](https://github.com/blue-build/cli/commit/e8e8bfa0966cd6b2bf827620aad6df7fb5339b36))
    - Don't build integration tests in +all ([`6a4c89d`](https://github.com/blue-build/cli/commit/6a4c89d567c4f7cd76415c56818527d9f7048bc2))
    - Release blue-build v0.5.2 ([`6fffe12`](https://github.com/blue-build/cli/commit/6fffe1286e91e62354fa6f19eb1a9220bb0852f5))
    - Manual bump of version ([`dbea80c`](https://github.com/blue-build/cli/commit/dbea80c94586b1d34e372639d01f4cf2baa0f2e7))
    - Manual update changelog for release ([`d005bfc`](https://github.com/blue-build/cli/commit/d005bfc9251313e96687754634bb42178e642a48))
    - Update changelog ([`e71b189`](https://github.com/blue-build/cli/commit/e71b1897d1e752008a407d27b40fd31c7465e6be))
    - Update Cargo.toml with new repo URL ([`4f62b3e`](https://github.com/blue-build/cli/commit/4f62b3e5df47b74191a4222a6d98c9ffc5d7a993))
    - Update outdated 60-custom.just ([`9454baa`](https://github.com/blue-build/cli/commit/9454baa75063b7c249fde08f663b5982c0572384))
</details>

## v0.5.1 (2024-01-22)

<csr-id-0573cc1d9ba1ab44721dd87f27bf4ad8bacd8634/>

### Bug Fixes

 - <csr-id-e325d5d3a13b351c0e2250fc89238a95688d5b88/> Allow single module from-file

### Other

 - <csr-id-0573cc1d9ba1ab44721dd87f27bf4ad8bacd8634/> Update README for upgrade and rebase commands

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.5.1 ([`5ece8aa`](https://github.com/blue-build/cli/commit/5ece8aa747f399025d576e43ffbcc485b934b2df))
    - Allow single module from-file ([`e325d5d`](https://github.com/blue-build/cli/commit/e325d5d3a13b351c0e2250fc89238a95688d5b88))
    - Update README for upgrade and rebase commands ([`0573cc1`](https://github.com/blue-build/cli/commit/0573cc1d9ba1ab44721dd87f27bf4ad8bacd8634))
</details>

## v0.5.0 (2024-01-21)

### New Features (BREAKING)

 - <csr-id-b547a326fd742e06bdea6b17f51afef459a609ad/> Upgrade and Rebase commands

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.5.0 ([`e2608b9`](https://github.com/blue-build/cli/commit/e2608b93ed7b310067888376f793acb555a5f6c5))
    - Upgrade and Rebase commands ([`b547a32`](https://github.com/blue-build/cli/commit/b547a326fd742e06bdea6b17f51afef459a609ad))
</details>

## v0.4.3 (2024-01-19)

<csr-id-0a780fb9aa31d6d2aedd88635a4a6f7083c9c3d7/>
<csr-id-1b950b08dc3ddf0146b86ae50ee21d0c7f427b57/>
<csr-id-218cc9c7d3a622183cfcf52d5f45cebc06f45349/>
<csr-id-5d50ac4fef235f95531a3a8f07870eabf153312c/>
<csr-id-fad8eb2ff9aa11c85192e5c9729f692b3e3bbefc/>
<csr-id-9636c2edc5f6612267392052a8e24da387c9ba60/>

### Chore

 - <csr-id-0a780fb9aa31d6d2aedd88635a4a6f7083c9c3d7/> Add CODEOWNERS file

### Other

 - <csr-id-1b950b08dc3ddf0146b86ae50ee21d0c7f427b57/> Use podman-api crate for building images
 - <csr-id-218cc9c7d3a622183cfcf52d5f45cebc06f45349/> use --privileged instead of WITH DOCKER
 - <csr-id-5d50ac4fef235f95531a3a8f07870eabf153312c/> Run both nightly and default integration tests
 - <csr-id-fad8eb2ff9aa11c85192e5c9729f692b3e3bbefc/> Enable integration tests

### Test

 - <csr-id-9636c2edc5f6612267392052a8e24da387c9ba60/> Add integration tests for build and template

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 4 calendar days.
 - 5 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.4.3 ([`c70d78c`](https://github.com/blue-build/cli/commit/c70d78c57c375f00a5d564576ba0adafbe849c40))
    - Use podman-api crate for building images ([`1b950b0`](https://github.com/blue-build/cli/commit/1b950b08dc3ddf0146b86ae50ee21d0c7f427b57))
    - Use --privileged instead of WITH DOCKER ([`218cc9c`](https://github.com/blue-build/cli/commit/218cc9c7d3a622183cfcf52d5f45cebc06f45349))
    - Run both nightly and default integration tests ([`5d50ac4`](https://github.com/blue-build/cli/commit/5d50ac4fef235f95531a3a8f07870eabf153312c))
    - Enable integration tests ([`fad8eb2`](https://github.com/blue-build/cli/commit/fad8eb2ff9aa11c85192e5c9729f692b3e3bbefc))
    - Add integration tests for build and template ([`9636c2e`](https://github.com/blue-build/cli/commit/9636c2edc5f6612267392052a8e24da387c9ba60))
    - Add CODEOWNERS file ([`0a780fb`](https://github.com/blue-build/cli/commit/0a780fb9aa31d6d2aedd88635a4a6f7083c9c3d7))
</details>

## v0.4.2 (2024-01-14)

### Bug Fixes

 - <csr-id-9ad018367e136bf75cfa614bc64c5f4499cbcab5/> Used wrong image for installer in Containerfile template

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.4.2 ([`d142ed7`](https://github.com/blue-build/cli/commit/d142ed77abe981d573a63347f4d3e980cf201fb1))
    - Used wrong image for installer in Containerfile template ([`9ad0183`](https://github.com/blue-build/cli/commit/9ad018367e136bf75cfa614bc64c5f4499cbcab5))
</details>

## v0.4.1 (2024-01-14)

### Documentation

 - <csr-id-41bdd85903e3ce7bb7254eeef20e402541bedb63/> Update README to describe using local builds

### Bug Fixes

 - <csr-id-f8dfc6b241922f018b123115b4d40f19002ccb16/> Installer used wrong image tag

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.4.1 ([`27d04e2`](https://github.com/blue-build/cli/commit/27d04e232e02f157e60c784e5d62df120a8e3a38))
    - Update README to describe using local builds ([`41bdd85`](https://github.com/blue-build/cli/commit/41bdd85903e3ce7bb7254eeef20e402541bedb63))
    - Installer used wrong image tag ([`f8dfc6b`](https://github.com/blue-build/cli/commit/f8dfc6b241922f018b123115b4d40f19002ccb16))
</details>

## v0.4.0 (2024-01-14)

### New Features (BREAKING)

 - <csr-id-754b4516e75e1b7483f85e5586a6ad637e3999d9/> remove containerfile arg since we use compiled time templates

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.4.0 ([`bf0479c`](https://github.com/blue-build/cli/commit/bf0479cc48737dd92565bf3a0a41e0badd3ba0b0))
    - Remove containerfile arg since we use compiled time templates ([`754b451`](https://github.com/blue-build/cli/commit/754b4516e75e1b7483f85e5586a6ad637e3999d9))
</details>

## v0.3.13 (2024-01-14)

### New Features

 - <csr-id-eaeb79f329282d9214ceda0b3d66b72f52dc2427/> Local image rebasing

### Bug Fixes

 - <csr-id-150aee028b611cf30415aa98ca5a2ee82c9ca550/> conflicting short args for build subcommand

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 7 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.3.13 ([`0611cea`](https://github.com/blue-build/cli/commit/0611cea4f58f6c180ddb578ce9252079d44b7bc8))
    - Conflicting short args for build subcommand ([`150aee0`](https://github.com/blue-build/cli/commit/150aee028b611cf30415aa98ca5a2ee82c9ca550))
    - Local image rebasing ([`eaeb79f`](https://github.com/blue-build/cli/commit/eaeb79f329282d9214ceda0b3d66b72f52dc2427))
</details>

## v0.3.12 (2024-01-06)

### Documentation

 - <csr-id-7d2a0780b0c86d89c8ae47b80f701b979c3d7dff/> Add logos

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.3.12 ([`7829ea6`](https://github.com/blue-build/cli/commit/7829ea6fe314a6f82611f146caebd8c00de28ae9))
    - Add logos ([`7d2a078`](https://github.com/blue-build/cli/commit/7d2a0780b0c86d89c8ae47b80f701b979c3d7dff))
</details>

## v0.3.11 (2024-01-04)

### Bug Fixes

 - <csr-id-ebd399e960edc3707f457807ca7a99c92a7c0ae7/> removed unwrap from template to handle with proper error message

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.3.11 ([`c13637c`](https://github.com/blue-build/cli/commit/c13637ca88056051f2cf7215b23cc6d32bc6e311))
    - Removed unwrap from template to handle with proper error message ([`ebd399e`](https://github.com/blue-build/cli/commit/ebd399e960edc3707f457807ca7a99c92a7c0ae7))
</details>

## v0.3.10 (2024-01-04)

<csr-id-d663b7574bb140848e1a80659440b3498444500b/>

### Bug Fixes

 - <csr-id-dfb315447c0d1bd41abe99fea25738e870eb97b8/> stop possible from-file, type module collision in template

### Refactor

 - <csr-id-d663b7574bb140848e1a80659440b3498444500b/> Use askama crate for compile-time template type checking

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.3.10 ([`7ae8dcd`](https://github.com/blue-build/cli/commit/7ae8dcd27372303ef64f850b4640b2a8a61d57d9))
    - Stop possible from-file, type module collision in template ([`dfb3154`](https://github.com/blue-build/cli/commit/dfb315447c0d1bd41abe99fea25738e870eb97b8))
    - Use askama crate for compile-time template type checking ([`d663b75`](https://github.com/blue-build/cli/commit/d663b7574bb140848e1a80659440b3498444500b))
</details>

## v0.3.9 (2024-01-01)

<csr-id-938ddae891b75049485949fa3f46cbc69b27f1af/>

### Bug Fixes

 - <csr-id-7dd3a8f0f9cbdc53d00d53e07f8548bc4ddcdddf/> clippy error for image_tag
 - <csr-id-ca95e3296d4cb199ebfbf7a7f39a2bdcc2f926f8/> Allow image_version to be a String
 - <csr-id-e0d93e81b51ce8baff9e2dc7da368695b71045ec/> Earthfile syntax error

### Refactor

 - <csr-id-938ddae891b75049485949fa3f46cbc69b27f1af/> inefficiency in generated Containerfile

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.3.9 ([`039c5f9`](https://github.com/blue-build/cli/commit/039c5f965947867c6501f147fc6d4f7ba6d234d3))
    - Inefficiency in generated Containerfile ([`938ddae`](https://github.com/blue-build/cli/commit/938ddae891b75049485949fa3f46cbc69b27f1af))
    - Clippy error for image_tag ([`7dd3a8f`](https://github.com/blue-build/cli/commit/7dd3a8f0f9cbdc53d00d53e07f8548bc4ddcdddf))
    - Allow image_version to be a String ([`ca95e32`](https://github.com/blue-build/cli/commit/ca95e3296d4cb199ebfbf7a7f39a2bdcc2f926f8))
    - Earthfile syntax error ([`e0d93e8`](https://github.com/blue-build/cli/commit/e0d93e81b51ce8baff9e2dc7da368695b71045ec))
</details>

## v0.3.8 (2023-12-30)

### Documentation

 - <csr-id-a2e5479d6594186b83576aa3911b818806e1810a/> renaming tool in docs

### Bug Fixes

 - <csr-id-d3ff4eed93ec0b23ef31476075fe8c5bb9683fa7/> rename ublue-rs to blue-build

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release blue-build v0.3.8 ([`3309ca5`](https://github.com/blue-build/cli/commit/3309ca5eccb4115e4b9b284ace30d53e23ce1392))
    - Rename ublue-rs to blue-build ([`d3ff4ee`](https://github.com/blue-build/cli/commit/d3ff4eed93ec0b23ef31476075fe8c5bb9683fa7))
    - Renaming tool in docs ([`a2e5479`](https://github.com/blue-build/cli/commit/a2e5479d6594186b83576aa3911b818806e1810a))
</details>

## v0.3.7 (2023-12-30)

### Bug Fixes

 - <csr-id-80477025825f9b8c01eea768950fc263876e64dd/> update README to point to new project

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release ublue-rs v0.3.7 ([`63cba6d`](https://github.com/blue-build/cli/commit/63cba6d27fd9cd6ec95fa8ec68f16998765151fc))
    - Update README to point to new project ([`8047702`](https://github.com/blue-build/cli/commit/80477025825f9b8c01eea768950fc263876e64dd))
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
    - Release ublue-rs v0.3.6 ([`ff3f066`](https://github.com/blue-build/cli/commit/ff3f066cbfe7471f1d1a59e937fe7427f4852f2b))
    - Bump version ([`88cc375`](https://github.com/blue-build/cli/commit/88cc37529a2c8b8d43e8610ea282c5c9951fa721))
    - Release ublue-rs v0.3.5 ([`9387d20`](https://github.com/blue-build/cli/commit/9387d2075f245ed596e3809dd9fb0be12e4a67a1))
    - Update cargo.toml ([`7bae446`](https://github.com/blue-build/cli/commit/7bae446d2782de142a38d45c60798860da76cc7a))
    - Fix changelog ([`bfdd3ce`](https://github.com/blue-build/cli/commit/bfdd3ce3330162cdb853427edaa6db3a913eaed4))
    - Logging ([`75dc311`](https://github.com/blue-build/cli/commit/75dc31182f4cf9aa9ee1d034fdf548163513a563))
    - Add Github support in Build command ([`6a15c56`](https://github.com/blue-build/cli/commit/6a15c56a906c51332aa5e5766c928692004ce97c))
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
    - Release ublue-rs v0.3.5 ([`386308f`](https://github.com/blue-build/cli/commit/386308f6492298aa0f81d0f5727b9adf6a3b7c22))
    - Update changelog for release ([`7897f9d`](https://github.com/blue-build/cli/commit/7897f9d6c09fae63961d7c4e8c44858eba97e204))
    - Add basic templating support for Github Actions ([`7ce7048`](https://github.com/blue-build/cli/commit/7ce70480bfc7e3bb669db0224cfa91bedf2bed6a))
    - Add support for alpine image and using either podman or buildah ([`3b07758`](https://github.com/blue-build/cli/commit/3b07758709e809f73749f652797ca382cfe526ac))
    - Add main README template ([`6c61cab`](https://github.com/blue-build/cli/commit/6c61cab07e1fce2373053b2f04034f4b2761284f))
    - Adding new subcommand ([`249f852`](https://github.com/blue-build/cli/commit/249f852a3faf8fcbde03bbc2cf7d9240a72940e3))
    - Adding more template files for init ([`556652f`](https://github.com/blue-build/cli/commit/556652f92a805209585aa8a76dbba98a12fdc5b6))
    - Update README and CHANGELOG ([`c559fb4`](https://github.com/blue-build/cli/commit/c559fb4d6b729b9e83ff9cfe94ded24b52949f94))
    - Add ability to use incremental caching for rust builds in Earthfile ([`a25e041`](https://github.com/blue-build/cli/commit/a25e0418c4d0fbd8193997f257a72e3b4eace06f))
    - Have ublue-cli manage iso-generator ([`a3da7e3`](https://github.com/blue-build/cli/commit/a3da7e3db025b2410aa767daee560109947e683c))
    - Switch to using typed builders ([`aa86f48`](https://github.com/blue-build/cli/commit/aa86f48a5d21c6d3358f1426486ca46d7ea98e72))
    - Fix SAVE IMAGE ([`e9cfc8a`](https://github.com/blue-build/cli/commit/e9cfc8ae06d7e0c6442922f153f16bd7a407688d))
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
    - Release ublue-rs v0.3.4 ([`f47dd02`](https://github.com/blue-build/cli/commit/f47dd02ea95c81ec004fcbc9e58240a95a6385c2))
    - Update changelog for release ([`785a60f`](https://github.com/blue-build/cli/commit/785a60ffd981e1be70aefdab5ee88aed76990d43))
    - Refactor Command Structs and create Earthly build ([`ebd861c`](https://github.com/blue-build/cli/commit/ebd861cd7eb5b3472b16bf9389270e33e5f981a0))
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
    - Release ublue-rs v0.3.3 ([`c20b917`](https://github.com/blue-build/cli/commit/c20b917728dfc7e71dc77b1d95ab63b76d528d20))
    - Update changelog for release ([`a5663af`](https://github.com/blue-build/cli/commit/a5663afd1bf7d976eda483232c4c76961cb31b33))
    - Set some env vars for cosign; force color logs ([`d936000`](https://github.com/blue-build/cli/commit/d9360005775ba6aaf99825a813a85fcedec0d23d))
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
    - Release ublue-rs v0.3.2 ([`f8214fc`](https://github.com/blue-build/cli/commit/f8214fc6236b63675d65a66865c9502d95303515))
    - Update changelog for release ([`211a393`](https://github.com/blue-build/cli/commit/211a393fdfdcbc362f4285c2a63a250b6aa9e6b2))
    - Improper trim of image digest ([`7f4f666`](https://github.com/blue-build/cli/commit/7f4f666b0aa82c6fe04c6c3a96630a59b7c1a675))
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
    - Release ublue-rs v0.3.1 ([`8878751`](https://github.com/blue-build/cli/commit/8878751989832d8a8f1bfbb69e0a4aace19d01ea))
    - Update changelog for release ([`73b9a1d`](https://github.com/blue-build/cli/commit/73b9a1ddb9e086e1716f658cf59f5b88c9051e68))
    - Remove single quotes from image_digest ([`b374d54`](https://github.com/blue-build/cli/commit/b374d546833eee10ccf9f7c8203fd5fe6637e143))
    - Add logging ([`b83cf57`](https://github.com/blue-build/cli/commit/b83cf574b8417a482c7f7fbfe9d3699aa9bd1d50))
    - Clippy ([`f437bda`](https://github.com/blue-build/cli/commit/f437bdaffa48d45e510a34cbd09d0ff736f92ded))
    - Add rusty-hook ([`5b1f997`](https://github.com/blue-build/cli/commit/5b1f99759c77377c8fbbe9c79b87d5c09c6479cd))
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
    - Release ublue-rs v0.3.0 ([`7745830`](https://github.com/blue-build/cli/commit/7745830345bf3c3c2ed97db085d5afabad1859a8))
    - Update changelog for release ([`985a3f6`](https://github.com/blue-build/cli/commit/985a3f6673e9d887b7bc761f20c68ae46be953d2))
    - Finish build feature ([`4ea6f77`](https://github.com/blue-build/cli/commit/4ea6f772e083419f74a5a304674b58c54a4c6a99))
    - Start work on build command ([`71d9397`](https://github.com/blue-build/cli/commit/71d93977b91cca8ad14bc1bc312988b042926333))
    - Update README ([`bcd7e71`](https://github.com/blue-build/cli/commit/bcd7e710a27ee9ef12417b4c17d99e728ecff2d0))
    - Remove legacy code" ([`785fc2f`](https://github.com/blue-build/cli/commit/785fc2f7621782a1597b728c5b5e49c41364e85e))
    - Move command structs into bin ([`006966b`](https://github.com/blue-build/cli/commit/006966bb351a34c435726dd7e202790001005a7c))
    - Make containerfile formatting nicer ([`49d512b`](https://github.com/blue-build/cli/commit/49d512b3f18db3370554c0e5b57b2dfc5d1d2abd))
    - Fix .git/ exclude ([`ea6143c`](https://github.com/blue-build/cli/commit/ea6143c7f7e628ea2958ccf8193c4e0e68595d2c))
    - Exclude some more files ([`13d10be`](https://github.com/blue-build/cli/commit/13d10bedf951d70a1f42c6dbebd0098ec7a2a610))
    - Add rust-toolchain.toml ([`1f030d6`](https://github.com/blue-build/cli/commit/1f030d6b74ebfb2f47b6772da59529be0996a2de))
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
    - Release ublue-rs v0.2.2 ([`df7cc56`](https://github.com/blue-build/cli/commit/df7cc56289361d08a965ff053d55cdf7afc8c656))
    - Update changelog for release ([`a811667`](https://github.com/blue-build/cli/commit/a8116677b382498585971d3dbd2124f46b90b020))
    - Fix version to match with published version ([`603a333`](https://github.com/blue-build/cli/commit/603a3335f9cd97a1905dae6c909f95bcff051686))
    - Update README, checking off a feature ([`33bee78`](https://github.com/blue-build/cli/commit/33bee78acfd25b89599b5e26646c9faeaf1576f8))
    - Comment out config for now ([`5968065`](https://github.com/blue-build/cli/commit/596806594df1d093ffaf77a45910a425f3917b2d))
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
    - Release ublue-rs v0.2.0 ([`6b0f684`](https://github.com/blue-build/cli/commit/6b0f6847e9d39110d563cf584cc41038729dc61b))
    - Print module context as json ([`c6f2e5b`](https://github.com/blue-build/cli/commit/c6f2e5b18de8a85e482583fed075fb25818d7f34))
    - Allow containerfile module to print out to main Containerfile ([`9564ca0`](https://github.com/blue-build/cli/commit/9564ca0af36cb9ba26ba6c158dd1e24db90deee5))
    - Remove tracing ([`52936ff`](https://github.com/blue-build/cli/commit/52936fffb195b837c9e93be4a99f9964fadae1e4))
    - Encapsulate module echo in quotes to be passed in as a single arg ([`f2ab9bf`](https://github.com/blue-build/cli/commit/f2ab9bfd4aeb98709a8fc8aaed7b535c3010a4ad))
    - Add support for legacy containerfiles and modules containerfiles ([`b1b2b0b`](https://github.com/blue-build/cli/commit/b1b2b0b2ac2be3655066317246486bc337f38ad4))
    - Support new modules based starting point template ([`85aadf7`](https://github.com/blue-build/cli/commit/85aadf73e520b5f0c14c2e98093745c45a52b0c1))
    - Create README ([`731e1d7`](https://github.com/blue-build/cli/commit/731e1d75671ebc764dbfa052c72b0df3fbd1141a))
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
    - Release ublue-rs v0.1.1 ([`0e4036b`](https://github.com/blue-build/cli/commit/0e4036ba86acc7859cbd9677090a7fb7ac2e8f07))
    - Release ublue-rs v0.1.1 ([`5d3756b`](https://github.com/blue-build/cli/commit/5d3756bfca4cb49ff14bccd009c73a34bcbd0f64))
    - Add changelog ([`b39fb4c`](https://github.com/blue-build/cli/commit/b39fb4cf1f19fee8ddd183a2401fe88143cf4dd7))
    - Revert back to published version number ([`acc29d6`](https://github.com/blue-build/cli/commit/acc29d60a6905eff92049738f0fcfffb9971770c))
    - Ignore the .sccache dir just in case ([`c6a339c`](https://github.com/blue-build/cli/commit/c6a339cdd088e21af9bfcb573697d936e86b4929))
    - Remove license-file prop ([`89acdbc`](https://github.com/blue-build/cli/commit/89acdbc6ec0430b6b35c62def6439b8f0a8a8c8c))
    - Include the cargo build pipeline ([`7f89c4e`](https://github.com/blue-build/cli/commit/7f89c4e266de5db5cc0921093c350ad4f2119b12))
    - Add features section ([`5c503ef`](https://github.com/blue-build/cli/commit/5c503eff4b46b95b70858ff4f4fac6459de25e53))
    - Put init and build behind feature flags ([`073ad4c`](https://github.com/blue-build/cli/commit/073ad4ca4a1d7e41583b4b0dcceb2a84a13f5a73))
    - Make changes to exclude and license ([`90bab6c`](https://github.com/blue-build/cli/commit/90bab6c9ff76b2c0c0e500ec4891513189eb5bb9))
    - Start work on pipeline ([`21beccd`](https://github.com/blue-build/cli/commit/21beccd018943cfb21fb2c07379aabbcfc3032fd))
    - Start work on init command ([`564ea91`](https://github.com/blue-build/cli/commit/564ea919a501264cb09c3181e0ed29e41bdc93ea))
    - Don't specify specific include ([`bc4557a`](https://github.com/blue-build/cli/commit/bc4557a9caae47169fc409db65422adb2a90b923))
    - Set bin ([`6b6578b`](https://github.com/blue-build/cli/commit/6b6578b3a7d3a7adebccc4cd13056745940521c4))
    - Remove default-run ([`fcf653d`](https://github.com/blue-build/cli/commit/fcf653d7f8d2c8d7db7b63967acf439f55a04d45))
    - Update Cargo.toml in order to publish ([`6753ab5`](https://github.com/blue-build/cli/commit/6753ab503a436207dc113101a36b79938fb7bd64))
    - Fix recipe and templates ([`8668a7a`](https://github.com/blue-build/cli/commit/8668a7a44247f8b253775c5b7e58c2096248a518))
    - Fix template ([`c415f6a`](https://github.com/blue-build/cli/commit/c415f6a90a1852a4c92d6b6cfd09eeeae3b84baf))
    - Clean up the code a bit ([`bd04489`](https://github.com/blue-build/cli/commit/bd04489dc3149d30a699db5732bf3bf9a61e0755))
    - Create autorun script capabilities ([`2cd8878`](https://github.com/blue-build/cli/commit/2cd887849012a705ee77adf0416b178963714a5f))
    - Add LICENSE ([`a28f0af`](https://github.com/blue-build/cli/commit/a28f0af02cfec48dadaabbfb8ca6951ddd2daa2e))
    - Allow for custom Containerfile adding ([`69effba`](https://github.com/blue-build/cli/commit/69effba45b7b1183c43606a5e67543b38f299a90))
    - Get cli in basic working order ([`bd6fabd`](https://github.com/blue-build/cli/commit/bd6fabd0de73217a69fbf89a8441de45b027c314))
    - Able to generate a Containerfile ([`e42cda0`](https://github.com/blue-build/cli/commit/e42cda01ff57b1717f514d42398aa9de7bbadc59))
    - Making some progress ([`5361b36`](https://github.com/blue-build/cli/commit/5361b36238a5e58f7d8604db9b0b4d9a0c0452b6))
    - Create templates, serialization structs, and cli arg parsing ([`783c53e`](https://github.com/blue-build/cli/commit/783c53ebb8118e4129eda6a395b047a25e0a1d1e))
    - Initial commit ([`6a7cadd`](https://github.com/blue-build/cli/commit/6a7cadd2f6d905580d5b4506ac186f9584e6b045))
</details>

