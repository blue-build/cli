#!/usr/bin/env bash

set -euo pipefail

# We use sudo for podman so that we can copy directly into /usr/local/bin

function cleanup() {
  echo "Cleaning up image"
  sudo podman stop -i -t 0 blue-build-installer
  sleep 2
  sudo podman image rm ghcr.io/blue-build/cli:latest-installer
  rm -f bb
}

trap cleanup SIGINT

sudo podman pull ghcr.io/blue-build/cli:latest-installer

sudo podman run --replace -d --rm --name blue-build-installer ghcr.io/blue-build/cli:latest-installer tail -f /dev/null

set +e
sudo podman cp blue-build-installer:/out/bb /usr/local/bin/bb

RETVAL=$?
set -e

if [ $RETVAL != 0 ]; then
  cleanup
  echo "Failed to copy file"
  exit 1
else
  # sudo mv bb /usr/local/bin/
  echo "Finished! BlueBuild has been installed at /usr/local/bin/bb"
  cleanup
fi

