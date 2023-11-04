# Ublue CLI

This is my personal project trying to create a more conise version of the [starting point](https://github.com/ublue-os/startingpoint/tree/template) repo all condensed into a single Rust based CLI tool.

## Installation

Right now the only way to install this tool is to use `cargo`.

```bash
cargo install --locked ublue-rs
```

### Legacy Starting Point

If you want to install the tool for use with the legacy setup of the starting point template, you can install it with:

```bash
cargo install --locked --features legacy --no-default-features ublue-rs
```

## How to use

Once you have the CLI tool installed, you can run the following to pull in your recipe file to generate a `Containerfile`.

```bash
ublue template -o <CONTAINERFILE> <RECIPE_FILE>
```

You can then use this with `podman` to build and publish your image. Further options can be viewed by running `ublue --help`

## Future Features

- [x] Update to the most recent stable style of the [starting point](https://github.com/ublue-os/startingpoint/tree/template) template
- [x] Setup pipeline automation for publishing
- [ ] Create an init command to create a repo for you to start out
- [ ] Setup the project to allow installing with `binstall`
- [ ] Create an install script for easy install for users without `cargo`
