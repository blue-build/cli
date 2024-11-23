#!/usr/bin/env bash

set -euo pipefail

if ! command -v jq > /dev/null; then
  rpm-ostree install jq
fi

ostree container commit
