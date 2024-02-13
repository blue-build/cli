#!/usr/bin/env bash

set -euo pipefail

VERSION=v0.7.0

# We use sudo for podman so that we can copy directly into /usr/local/bin

function cleanup() {
  echo "Cleaning up image"
  sudo podman stop -i -t 0 blue-build-installer
  sleep 2
  sudo podman image rm ghcr.io/blue-build/cli:${VERSION}-installer
}

trap cleanup SIGINT

sudo podman run \
  --pull always \
  --replace \
  --detach \
  --rm \
  --name blue-build-installer \
  ghcr.io/blue-build/cli:${VERSION}-installer \
  tail -f /dev/null

set +e
sudo podman cp blue-build-installer:/out/bluebuild /usr/local/bin/bluebuild

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

