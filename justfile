#!/usr/bin/env just --justfile

export RUST_BACKTRACE := "1"

set dotenv-load := true
set shell := ["bash", "-xeu", "-o", "pipefail", "-c"]
set positional-arguments := true

# default recipe to display help information
default:
  @just --list

# release: Run cargo release and push the tag separately
release *args:
  # --workspace: updating all crates in the workspace
  # --no-tag: do not push tag for each new version
  # --no-confirm: don't look for user input, just run the command
  # --execute: not a dry run
  cargo release $1 -v \
    --workspace \
    --no-tag \
    --no-confirm \
    --execute

  VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "blue-build") .version')
  echo "Pushing tag: v${VERSION}"
  git tag "v${VERSION}"
  git push origin "v${VERSION}"
