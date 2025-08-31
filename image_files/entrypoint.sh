#!/usr/bin/env bash

set -euo pipefail

# mkdir -p /home/bluebuild/.local/share/containers/storage/networks/
# cp /etc/containers/networks/podman.json /home/bluebuild/.local/share/containers/storage/networks/podman.json
# # podman network inspect podman | jq .[] > /home/bluebuild/.local/share/containers/storage/networks/podman.json
# chown -R bluebuild:bluebuild /home/bluebuild/.local/share/containers

# exec runuser -u bluebuild -- dumb-init "$@"
dumb-init "$@"
