#!/bin/sh

# TODO: Remove once we're all POSIX https://github.com/blue-build/modules/issues/503
command_exists() {
    [ -x "/usr/bin/$1" ] || [ -x "/bin/$1" ] || [ -x "/usr/sbin/$1" ] || [ -x "/sbin/$1" ]
}

if ! command_exists bash \
    || ! command_exists jq \
    || ! command_exists curl \
    || ! command_exists grep \
    || ! command_exists ls; then
    if command_exists dnf5; then
        dnf5 -y install bash curl coreutils jq grep
    elif command_exists dnf4; then
        dnf4 -y install bash curl coreutils jq grep
    elif command_exists zypper; then
        zypper --non-interactive install --auto-agree-with-licenses bash curl coreutils jq grep find
    elif command_exists pacman; then
        pacman --sync --noconfirm --refresh --sysupgrade bash curl coreutils jq grep
    elif command_exists apt-get; then
        apt-get update
        DEBIAN_FRONTEND=noninteractive apt-get -y install bash curl coreutils jq grep
    elif command_exists apk; then
        apk add --no-cache bash curl coreutils grep jq
    fi
fi
