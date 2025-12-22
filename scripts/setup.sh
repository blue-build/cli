#!/bin/sh

# TODO: Remove once we're all POSIX https://github.com/blue-build/modules/issues/503
if [ -x /usr/bin/dnf5 ] \
    || [ -x /bin/dnf5 ] \
    || [ -x /sbin/dnf5 ]; then
    dnf5 -y install bash curl wget coreutils jq grep
elif [ -x /usr/bin/dnf4 ] \
    || [ -x /bin/dnf4 ] \
    || [ -x /sbin/dnf4 ]; then
    dnf4 -y install bash curl wget coreutils jq grep
elif [ -x /usr/bin/zypper ] \
    || [ -x /bin/zypper ] \
    || [ -x /sbin/zypper ]; then
    zypper --non-interactive install --auto-agree-with-licenses bash curl wget coreutils jq grep find
elif [ -x /usr/bin/pacman ] \
    || [ -x /bin/pacman ] \
    || [ -x /sbin/pacman ]; then
    pacman --sync --noconfirm --refresh --sysupgrade bash curl wget coreutils jq grep
elif [ -x /usr/bin/apt-get ] \
    || [ -x /bin/apt-get ] \
    || [ -x /sbin/apt-get ]; then
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get -y install bash curl wget coreutils jq grep
elif [ -x /usr/bin/apk ] \
    || [ -x /bin/apk ] \
    || [ -x /sbin/apk ]; then
    apk add --no-cache bash curl coreutils wget grep jq
fi
