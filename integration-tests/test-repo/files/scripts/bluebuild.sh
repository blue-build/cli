#!/usr/bin/env bash

set -euo pipefail

cargo install blue-build --debug --all-features --target x86_64-unknown-linux-gnu
mkdir -p /out/
mv $CARGO_HOME/bin/bluebuild /out/bluebuild
