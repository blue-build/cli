#!/usr/bin/env bash

set -euo pipefail

rm -rf /tmp/* /var/* /usr/etc

if command -v bootc > /dev/null; then
  bootc container lint
fi
