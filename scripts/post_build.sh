#!/usr/bin/env bash

set -euo pipefail

rm -rf /tmp/* /var/*
ostree container commit
