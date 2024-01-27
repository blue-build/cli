#!/usr/bin/env just --justfile

export RUST_BACKTRACE := "1"

set shell := ["bash", "-cu"] 
set dotenv-load := true

# default recipe to display help information
default:
  @just --list

cargo_bump_release_test:
    #!/usr/bin/env bash
    set -euxo pipefail

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

    # See release.yml for meaning of these arguments
    cargo release "${next_version}" \
    --workspace \
    --no-publish \
    --no-tag \
    --no-confirm \
    --no-push