#!/usr/bin/env just --justfile

export RUST_BACKTRACE := "1"

set dotenv-load := true
set shell := ["bash", "-cu"]
set positional-arguments := true

# default recipe to display help information
default:
  @just --list

# release: remove the dev suffix, like going from 0.X.0-dev to 0.X.0
# --workspace: updating all crates in the workspace
# --no-publish: do not publish to crates.io
# --execute: not a dry run
# --no-tag: do not push tag for each new version
# --no-push: do not push the update commits
# --dependent-version upgrade: change 0.X.0-dev in internal dependencies to 0.X.0
# --exclude: ignore those packages
cargo-release *args:
  #!/usr/bin/env bash
  set -euo pipefail

  cargo release release -v \
    --workspace \
    --no-publish \
    --no-tag \
    --no-confirm \
    --no-push \
    --dependent-version upgrade "$@"

# See @cargo-release for meaning of cargo-release arguments
cargo-post-release *args:
  #!/usr/bin/env bash
  set -euo pipefail

  # Read the current version from Cargo.toml
  current_version=$(cargo metadata --format-version 1 --no-deps | \
  jq --raw-output '.packages | .[] | select(.name == "blue-build").version')

  echo "Current Version: $current_version"

  # Sanity check: current version should be 0.X.Y
  if ! grep -q '^0\.[0-9]\+\.[0-9]\+$' <<< "${current_version}"; then
  echo "Invalid version (not in 0.X.Y format): ${current_version}"
  exit 1
  fi

  minor_version=$(sed 's/^0\.\([0-9]\+\).*/\1/' <<< "${current_version}")
  next_version=0.$((minor_version + 1)).0-dev
  echo "Bumping version to ${next_version}"

  # See @cargo-release for meaning of these arguments
  cargo release -v "${next_version}" \
  --workspace \
  --no-publish \
  --no-tag \
  --no-confirm \
  --no-push "$@"
