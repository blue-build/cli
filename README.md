<div align="center">
  <center>
    <img src="https://github.com/blue-build/.github/assets/60004820/337323ed-70e4-4025-8c73-e8fe0c183c7c" alt="BlueBuild. A minimal logo with a blue-billed duck holding a golden wrench in its beak." style="max-height: 300px;" />
  </center>
</div>

# BlueBuild

BlueBuild's command line program that builds Containerfiles and custom images based on your recipe.yml.

## Installation

### Distrobox

We package a `fedora-toolbox` and `alpine` image with all the tools needed to run `bluebuild`. You can use `distrobox` to run the application without needing to install it on your machine.

```bash
distrobox create blue-build --image ghcr.io/blue-build/cli
distrobox enter blue-build
```

### Cargo

This is the best way to install as it gives you the opportunity to bulid for your specific environment.

```bash
cargo install --locked blue-build
```

### Podman/Docker

This will install the binary on your system in `/usr/local/bin`. This is only a `linux-gnu` version.

```bash
podman run --rm ghcr.io/blue-build/cli:latest-installer | bash
```

## How to use

### Templating

Once you have the CLI tool installed, you can run the following to pull in your recipe file to generate a `Containerfile`.

```bash
bluebuild template -o <CONTAINERFILE> <RECIPE_FILE>
```

You can then use this with `podman` or `buildah` to build and publish your image. Further options can be viewed by running `bluebuild template --help`

### Building

If you don't care about the details of the template, you can run the `build` command.

```bash
bluebuild build ./config/recipe.yaml
```

This will template out the file and build with `buildah` or `podman`. 

#### Local Builds

##### Rebase

If you want to test your changes, you can do so by using the `rebase` command. This will create an image as a `.tar.gz` file, store it in `/etc/blue-build`, an run `rpm-ostree rebase` on that newly built file.

```bash
sudo bluebuild rebase config/recipe.yml
```

You can initiate an immediate restart by adding the `--reboot/-r` option.

##### Upgrade

When you've rebased onto a local image archive, you can update your image for your recipe by running:

```bash
sudo bluebuild upgrade config/recipe.yml
```

The `--reboot` argument can be used with this command as well.

#### CI Builds

##### GitHub

You can use our [GitHub Action](https://github.com/blue-build/github-action) by using the following `.github/workflows/build.yaml`:

```yaml
name: bluebuild
on:
  schedule:
    - cron: "00 17 * * *" # build at 17:00 UTC every day 
                          # (20 minutes after last ublue images start building)
  push:
    paths-ignore: # don't rebuild if only documentation has changed
      - "**.md"
  pull_request:
  workflow_dispatch: # allow manually triggering builds
jobs:
  bluebuild:
    name: Build Custom Image
    runs-on: ubuntu-22.04
    permissions:
      contents: read
      packages: write
      id-token: write
    strategy:
      fail-fast: false # stop GH from cancelling all matrix builds if one fails
      matrix:
        recipe:
          # !! Add your recipes here 
          - recipe.yml
    steps:
       # the build is fully handled by the reusable github action
      - name: Build Custom Image
        uses: blue-build/github-action@v1.0.0
        with:
          recipe: ${{ matrix.recipe }}
          cosign_private_key: ${{ secrets.SIGNING_SECRET }}
          registry_token: ${{ github.token }}
          pr_event_number: ${{ github.event.number }}
 ```

##### Gitlab

If you're running in Gitlab CI, it will automatically sign your image using Gitlab's own OIDC service. Here's an example of a `.gitlab-ci.yaml`:

```yaml
workflow:
  rules:
    - if: $CI_COMMIT_BRANCH && $CI_OPEN_MERGE_REQUESTS && $CI_PIPELINE_SOURCE == "push"
      when: never
    - if: "$CI_COMMIT_TAG"
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: "$CI_COMMIT_BRANCH && $CI_OPEN_MERGE_REQUESTS"
      when: never
    - if: "$CI_COMMIT_BRANCH"
stages:
  - build
variables:
  ACTION:
    description: "Action to perform for the pipeline."
    value: "build-image"
    options:
      - "build-image"
build-image:
  stage: build
  image: ghcr.io/blue-build/cli:latest-alpine
  retry: 2
  rules:
    - if: $ACTION == "build-image"
  parallel:
    matrix:
      - RECIPE:
          - recipe.yml
  id_tokens:
    SIGSTORE_ID_TOKEN:
      aud: sigstore
  script:
    - bluebuild build --push ./config/$RECIPE
```

## Future Features

- [x] Update to the most recent stable style of the [starting point](https://github.com/ublue-os/startingpoint/tree/template) template
- [x] Setup pipeline automation for publishing
- [ ] Create an init command to create a repo for you to start out
- [ ] Setup the project to allow installing with `binstall`
- [x] Create an install script for easy install for users without `cargo`
