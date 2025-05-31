#!/usr/bin/env bash

set -euo pipefail
. /scripts/exports.sh

rm -rf /tmp/* /var/*

if feature_enabled "bootc" && command -v bootc > /dev/null; then
  bootc container lint
fi
