export RUST_BACKTRACE := "1"
export BB_CACHE_LAYERS := "true"
export TEST_SECRET := "test123"
export BB_SKIP_VALIDATION := "true"

set dotenv-load := true
set positional-arguments := true

# default recipe to display help information
default:
  @just --list

# Clean up development files and images
clean:
  cargo clean
  command -v docker \
    && docker buildx --builder bluebuild prune -f \
    && docker system prune -f \
    || true
  command -v podman \
    && podman system prune -f \
    || true
  command -v earthly \
    && earthly prune --reset \
    || true

# Install bluebuild using cargo with release optimization
install:
  cargo install --path . --locked

# Install bluebuild with all features with release optimizations
install-all-features:
  cargo install --all-features --path . --locked

# Install bluebuild using cargo with debug targets
install-debug:
  cargo install --debug --path . --locked

# Install bluebuild with all features and debug target
install-debug-all-features:
  cargo install --debug --all-features --path . --locked

# Run unit tests
test:
  cargo test --workspace -- --show-output

# Run unit tests for all features
test-all-features:
  cargo test --workspace --all-features -- --show-output

# Run clippy
lint:
  cargo clippy

# Run clippy for all features
lint-all-features:
  cargo clippy --all-features

# Watch the files and run cargo check on changes
watch:
  cargo watch -c

# Install bluebuild whenever there is a change in the project files
watch-install:
  cargo watch -c -x 'install --debug --locked --path .'

# Install bluebuild whenever there is a change in the project files
watch-install-all-features:
  cargo watch -c -x 'install --debug --locked --all-features --path .'

# Run tests anytime a file is changed
watch-test:
  cargo watch -c -x 'test --workspace -- --show-output'

# Run all feature tests anytime a file is changed
watch-test-all-features:
  cargo watch -c -x 'test --workspace --all-features -- --show-output'

# Run lint anytime a file is changed
watch-lint:
  cargo watch -c -x 'clippy'

# Run all feature lint anytime a file is changed
watch-lint-all-features:
  cargo watch -c -x 'clippy --all-features'

# Expand the macros of a module for debugging
expand *args:
  cargo expand $@ > ./expand.rs
  $EDITOR ./expand.rs

# Installs cargo tools that help with development
tools:
  rustup toolchain install stable
  rustup override set stable
  rustup component add --toolchain stable rust-analyzer clippy rustfmt
  cargo install --locked cargo-watch cargo-expand bacon

# Run cargo release and push the tag separately
release *args:
  #!/usr/bin/env bash
  set -euxo pipefail

  earthly --ci +run-checks

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
  git tag -m "v${VERSION}" "v${VERSION}"
  git push origin "v${VERSION}"
  gh release create --generate-notes --latest "v${VERSION}"

should_push := if env('GITHUB_ACTIONS', '') != '' {
  if env('COSIGN_PRIVATE_KEY', '') != '' {
    '--push'
  } else {
    ''
  }
} else {
  ''
}

cargo_bin := if env('CARGO_HOME', '') != '' {
  x"${CARGO_HOME:-}/bin"
} else {
  x"$HOME/.cargo/bin"
}

generate-test-secret:
  mkdir -p integration-tests/test-repo/secrets
  echo "321tset" > integration-tests/test-repo/secrets/test-secret

# Run all integration tests
integration-tests: generate-test-secret test-docker-build test-empty-files-build test-arm64-build test-podman-build test-buildah-build test-generate-iso-image test-generate-iso-recipe test-multiplatform

# Run docker driver integration test
test-docker-build: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    -B docker \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe.yml recipes/recipe-gts.yml

test-empty-files-build: generate-test-secret install-debug-all-features
  cd integration-tests/empty-files-repo \
  && bluebuild build \
    --retry-push \
    -B docker \
    -S sigstore \
    {{ should_push }} \
    -vv

test-rechunk-build: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    {{ should_push }} \
    -vv \
    --rechunk \
    recipes/recipe-rechunk.yml

test-fresh-rechunk-build: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    {{ should_push }} \
    -vv \
    --rechunk \
    --rechunk-clear-plan \
    recipes/recipe-rechunk.yml

# Run arm integration test
test-arm64-build: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    --platform linux/arm64 \
    {{ should_push }} \
    -vv \
    recipes/recipe-arm64.yml

# Run docker driver external login integration test
test-docker-build-external-login: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe-docker-external.yml

# Run podman driver integration test
test-podman-build: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    -B podman \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe-podman.yml

# Run buildah driver integration test
test-buildah-build: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    -B buildah \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe-buildah.yml

# Run the multi-platform builds
test-multiplatform: test-multiplatform-docker test-multiplatform-podman test-multiplatform-buildah test-multiplatform-rechunk

test-multiplatform-docker: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    -B docker \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe-multiplatform-docker.yml

test-multiplatform-podman: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    -B podman \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe-multiplatform-podman.yml

test-multiplatform-buildah: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    -B buildah \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe-multiplatform-buildah.yml

test-multiplatform-rechunk: generate-test-secret install-debug-all-features
  cd integration-tests/test-repo \
  && bluebuild build \
    --retry-push \
    --rechunk \
    -S sigstore \
    {{ should_push }} \
    -vv \
    recipes/recipe-multiplatform-rechunk.yml

# Run ISO generator for images
test-generate-iso-image: generate-test-secret install-debug-all-features
  #!/usr/bin/env bash
  set -eu
  ISO_OUT=$(mktemp -d)
  bluebuild generate-iso -vv --output-dir "$ISO_OUT" image ghcr.io/blue-build/cli/test:40

# Run ISO generator for images
test-generate-iso-recipe: generate-test-secret install-debug-all-features
  #!/usr/bin/env bash
  set -eu
  ISO_OUT=$(mktemp -d)
  cd integration-tests/test-repo
  bluebuild generate-iso -vv --output-dir "$ISO_OUT" recipe recipes/recipe.yml

# Build a local cli image
build-local-cli-image:
  earthly --ci --output -P +blue-build-cli --RELEASE='false'

git_sha := `git rev-parse HEAD`
tty_arg := `[ -t 0 ] && echo "t" || echo ""`

# Run a command in the cli container
exec-cli-container +args: build-local-cli-image
  docker run -i{{ tty_arg }} --privileged --rm \
    -v ./integration-tests/test-repo:/bluebuild \
    -e TEST_SECRET="$TEST_SECRET" \
    ghcr.io/blue-build/cli:{{ git_sha }} \
    {{ args }}

# Run a cli container using the podman build driver
test-container-podman-build: \
  generate-test-secret \
  (exec-cli-container "bluebuild" "build" "-B" "podman" "--squash" "-vv")

# Run a cli container using the podman build driver with rechunk
test-container-podman-rechunk: \
  generate-test-secret \
  (exec-cli-container "bluebuild" "build" "-B" \
    "podman" "-vv" "--rechunk" "recipes/recipe-rechunk.yml")
