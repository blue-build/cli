[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/blue-build/cli/badge)](https://scorecard.dev/viewer/?uri=github.com/blue-build/cli)

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
- Buildah - v1.29 and above

## Installation

Every image created with `bluebuild` comes with the CLI installed. If you have not built and booted a `bluebuild` created image, you can follow these instructions to install it.

### Cargo

This is the best way to install as it gives you the opportunity to build for your specific environment.

```bash
cargo install --locked blue-build
```

### Podman/Docker

This will install the binary on your system in `/usr/local/bin`.

```bash
podman run --pull always --rm ghcr.io/blue-build/cli:latest-installer | bash
```

```bash
docker run --pull always --rm ghcr.io/blue-build/cli:latest-installer | bash
```

### Github Install Script

```bash
bash <(curl -s https://raw.githubusercontent.com/blue-build/cli/main/install.sh)
```

### Distrobox

We package an `alpine` image with all the tools needed to run `bluebuild`. You can use `distrobox` to run the application without needing to install it on your machine. You can clone this repo locally and run:

```bash
distrobox assemble create
```

This will export `bluebuild` to your local machine and allow you to build images and test out your recipes. For security reasons, we keep this as a rootless image which means you will not be able to use this method to locally rebase to an image. If you want that capability, you should install the CLI tool directly.

Refer to the [distrobox documentation](https://distrobox.it/usage/distrobox-export/) for more information.

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

## How to use

### Generating `Containerfile`

Once you have the CLI tool installed, you can run the following to pull in your recipe file to generate a `Containerfile`.

```bash
bluebuild generate -o <CONTAINERFILE> <RECIPE_FILE>
```

You can then use this with `docker`, `podman`, or `buildah` to build and publish your image. Further options can be viewed by running `bluebuild template --help`

### Building

If you don't care about the details of the template, you can run the `build` command.

```bash
bluebuild build ./recipes/recipe.yml
```

This will template out the file and build with `docker`, `podman`, or `buildah`.

### Completions

The `bluebuild completions` command generates shell completions, printed to stdout. These completions can be stored for integration in your shell environment. For example, on a system with [bash-completion](https://github.com/scop/bash-completion/) installed:

```bash
# user completions
$ bluebuild completions bash > ~/.local/share/bash-completion/completions/bluebuild
# system-wide completions
$ bluebuild completions bash | sudo tee /usr/share/bash-completion/completions/bluebuild
```

Subsequent invocations of `bluebuild` will respond to `<Tab>` autocompletions:

```bash
$ bluebuild # press <Tab>
-v           -V           --help       template     bug-report
-q           --verbose    --version    upgrade      completions
-h           --quiet      build        rebase       help
```

Currently, bluebuild completions are available for `bash`, `zsh`, `fish`, `powershell`, `nushell`, and `elvish` shell environments. Please follow your shell's documentation for completion scripts.

#### Local Builds

##### Switch

With the switch command, you can build and boot an image locally using an `oci-archive` tarball. The `switch` command can be run as a normal user and will only ask for `sudo` permissions when moving the archive into `/etc/bluebuild`.

```bash
bluebuild switch recipes/recipe.yml
```

You can initiate an immediate restart by adding the `--reboot/-r` option.

#### CI Builds

##### GitHub

You can use our [GitHub Action](https://github.com/blue-build/github-action) by using the following `.github/workflows/build.yml`:

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
        uses: blue-build/github-action@v1
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
    name: ghcr.io/blue-build/cli
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
    - bluebuild build --push ./recipes/$RECIPE
```
