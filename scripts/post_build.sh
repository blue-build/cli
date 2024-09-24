#!/usr/bin/env bash

set -euo pipefail

if command -v bootupctl && [ -f /usr/lib/ostree-boot/efi/EFI ] && [ "$OS_VERSION" -ge "40" ]; then
  echo "Generating update metadata"
  bootupctl backend generate-update-metadata
else
  echo "Program bootupctl not installed or EFI file not available, skipping..."
fi

rm -fr /tmp/* /var/*
ostree container commit
