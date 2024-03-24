<div align="center">
  <center>
    <img src="https://github.com/blue-build/.github/assets/60004820/337323ed-70e4-4025-8c73-e8fe0c183c7c" alt="BlueBuild. A minimal logo with a blue-billed duck holding a golden wrench in its beak." style="max-height: 300px;" />
  </center>
</div>

# BlueBuild

BlueBuild's command line program that builds Containerfiles and custom images based on your recipe.yml.

## Requirements

The `bluebuild` tool takes advantage of newer build features. Specifically bind, cache, and tmpfs mounts on the `RUN` instructions. We support using the following tools and their versions:

- Docker - v23 and above
- Podman - v4 and above
- Buildah - v1.24 and above

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

### Nix Flake

You can install this CLI through the Nix flake on [Flakehub](https://flakehub.com/)

#### Non-nixos

You can install BlueBuild to your global package environment on non-nixos systems by running

```shell
# you can replace "*" with a specific tag
nix profile install https://flakehub.com/f/bluebuild/cli/*.tar.gz#bluebuild
```

#### NixOS

If you are using a dedicated flake to manage your dependencies, you can add BlueBuild as a flake input throught the [fh](https://github.com/DeterminateSystems/fh) cli (that can be installed through nixpkgs) and add `bluebuild` to it.
```nix
{pkgs,inputs,...}: {
    ...
    environment.SystemPackages = [
        inputs.bluebuild.packages.${pkgs.system}.bluebuild # change bluebuild with the fh added input name 
    ];
    ...
}
```

If you are not using a dedicated nix flake, you can add the BlueBuild flake as a variable inside your `/etc/nixos/*.nix` configuration, though this requires you to run `nixos-rebuild` with the `--impure` variable, it is not advisable to do so.

```nix
{pkgs,...}:
let
    bluebuild = builtins.fetchTarball "https://flakehub.com/f/bluebuild/cli/*.tar.gz";
in {
    ...
    environment.SystemPackages = [
        bluebuild.packages.${pkgs.system}.bluebuild
    ];
    ...
}
```

You can also use `nix develop .#` in this repos directory to run a nix shell with development dependencies and some helful utilities for building BlueBuild! 

### Github Install Script

```bash
bash <(curl -s https://raw.githubusercontent.com/blue-build/cli/main/install.sh)
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

If you want to test your changes, you can do so by using the `rebase` command. This will create an image as a `.tar.gz` file, store it in `/etc/bluebuild`, an run `rpm-ostree rebase` on that newly built file.

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
    runs-on: ubuntu-latest
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

We also support GitLab CI! Fun fact, this project started out as a way to build these images in GitLab. You will want to make use of GitLab's [Secure Files](https://docs.gitlab.com/ee/ci/secure_files/index.html) feature for using your cosign private key for signing. Here's an example of a `.gitlab-ci.yml`:

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

build-image:
  stage: build
  image: 
    name: ghcr.io/blue-build/cli:main
    entrypoint: [""]
  services:
    - docker:dind
  parallel:
    matrix:
      - RECIPE:
          # Add your recipe files here
          - recipe.yml
  variables:
    # Setup a secure connection with docker-in-docker service
    # https://docs.gitlab.com/ee/ci/docker/using_docker_build.html
    DOCKER_HOST: tcp://docker:2376
    DOCKER_TLS_CERTDIR: /certs
    DOCKER_TLS_VERIFY: 1
    DOCKER_CERT_PATH: $DOCKER_TLS_CERTDIR/client
  before_script:
    # Pulls secure files into the build
    - curl --silent "https://gitlab.com/gitlab-org/incubation-engineering/mobile-devops/download-secure-files/-/raw/main/installer" | bash
    - export COSIGN_PRIVATE_KEY=$(cat .secure_files/cosign.key)
  script:
    - sleep 5 # Wait a bit for the docker-in-docker service to start
    - bluebuild build --push ./config/$RECIPE
```

## Future Features

- Stages for parallel building (useful for compiling programs for your image)
- Automatic download and management of image keys for seemless signed image rebasing
- Module command for easy 3rd party plugin management
- Create an init command to create a repo for you to start out
- Setup the project to allow installing with `cargo-binstall`
