#!/usr/bin/env bash

set -euo pipefail

VERSION=v0.9.17

# Container runtime
function cr() {
  if command -v podman > /dev/null; then
    podman $@
  elif command -v docker > /dev/null; then
    docker $@
  else
    echo "Need docker or podman to install!!"
    exit 1
  fi
}

# We use sudo for podman so that we can copy directly into /usr/local/bin
function cleanup() {
  echo "Cleaning up image"
  cr rm blue-build-installer
  sleep 2
  cr image rm ghcr.io/blue-build/cli:${VERSION}-installer
}

trap cleanup SIGINT

cr create \
  --pull always \
  --replace \
  --name blue-build-installer \
  ghcr.io/blue-build/cli:${VERSION}-installer

set +e
cr cp blue-build-installer:/out/bluebuild /tmp/

sudo mv /tmp/bluebuild /usr/local/bin/

RETVAL=$?
set -e

if [ $RETVAL != 0 ]; then
  cleanup
  echo "Failed to copy file"
  exit 1
else
  # sudo mv bluebuild /usr/local/bin/
  echo "Finished! BlueBuild has been installed at /usr/local/bin/bluebuild"
  cleanup
fi

