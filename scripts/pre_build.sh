#!/usr/bin/env bash

set -euo pipefail

if ! command -v jq > /dev/null; then
  if command -v rpm-ostree > /dev/null; then
    rpm-ostree install jq
  else
    dnf -y install jq
  fi
fi
