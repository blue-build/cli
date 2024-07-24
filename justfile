#!/usr/bin/env just --justfile

export RUST_BACKTRACE := "1"

set dotenv-load := true
set positional-arguments := true

# default recipe to display help information
default:
  @just --list

# Install bluebuild using cargo with release optimization
install:
  cargo install --path .

# Install bluebuild with all features with release optimizations
install-all-features:
  cargo install --all-features --path .

# Install bluebuild using cargo with debug targets
install-debug:
  cargo install --debug --path .

# Install bluebuild with all features and debug target
install-debug-all-features:
  cargo install --debug --all-features --path .

# Run unit tests
test:
  cargo test --workspace -- --show-output

# Run unit tests for all features
test-all-features:
  cargo test --workspace --all-features -- --show-output

# Run clippy
lint:
  cargo clippy -- -D warnings

# Run clippy for all features
lint-all-features:
  cargo clippy --all-features -- -D warnings

# Watch the files and run cargo check on changes
watch:
  cargo watch -c

# Install bluebuild whenever there is a change in the project files
watch-install:
  cargo watch -c -x 'install --debug --path .'

# Install bluebuild whenever there is a change in the project files
watch-install-all-features:
  cargo watch -c -x 'install --debug --all-features --path .'

# Run tests anytime a file is changed
watch-test:
  cargo watch -c -x 'test --workspace -- --show-output'

# Run all feature tests anytime a file is changed
watch-test-all-features:
  cargo watch -c -x 'test --workspace --all-features -- --show-output'

# Run lint anytime a file is changed
watch-lint:
  cargo watch -c -x 'clippy -- -D warnings'

# Run all feature lint anytime a file is changed
watch-lint-all-features:
  cargo watch -c -x 'clippy --all-features -- -D warnings'

# Installs cargo tools that help with development
tools:
  cargo install cargo-watch

# Run cargo release and push the tag separately
release *args:
  #!/usr/bin/env bash
  set -euxo pipefail
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
  gh release create --generate-notes --latest "v${VERSION}"
