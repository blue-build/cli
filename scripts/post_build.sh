#!/usr/bin/env bash

set -euo pipefail

if ! command -v bootc; then
  rpm-ostree install bootc
fi

bootupctl backend generate-update-metadata
rm -fr /tmp/* /var/*
ostree container commit
