#!/usr/bin/env bash

set -euo pipefail

if command -v bootc; then
  bootupctl backend generate-update-metadata
fi

rm -fr /tmp/* /var/*
ostree container commit
