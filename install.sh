#!/usr/bin/env bash

set -euo pipefail

function cleanup() {
  echo "Cleaning up image"
  podman stop -i -t 0 blue-build-installer
  sleep 2
  podman image rm registry.gitlab.com/wunker-bunker/blue-build:installer
}

podman pull registry.gitlab.com/wunker-bunker/blue-build:installer

podman run -d --rm --name blue-build-installer registry.gitlab.com/wunker-bunker/blue-build:installer tail -f /dev/null

set +e
podman cp blue-build-installer:/out/bb /usr/local/bin/bb

RETVAL=$?
set -e

if [ -n $RETVAL ]; then
  cleanup
  echo "Failed to copy file, try:"
  printf "\tpodman run --rm registry.gitlab.com/wunker-bunker/blue-build:installer | sudo bash\n"
  exit 1
else
  cleanup
fi

