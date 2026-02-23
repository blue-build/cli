#!/bin/sh

# TODO: Remove once we're all POSIX https://github.com/blue-build/modules/issues/503
bash_exists() {
    [ -x /usr/bin/bash ] || [ -x /bin/bash ] || [ -x /sbin/bash ]
}

curl_exists() {
    [ -x /usr/bin/curl ] || [ -x /bin/curl ] || [ -x /sbin/curl ]
}

jq_exists() {
    [ -x /usr/bin/jq ] || [ -x /bin/jq ] || [ -x /sbin/jq ]
}

grep_exists() {
    [ -x /usr/bin/grep ] || [ -x /bin/grep ] || [ -x /sbin/grep ]
}

coreutils_exists() {
    [ -x /usr/bin/ls ] || [ -x /bin/ls ] || [ -x /sbin/ls ]
}

if ! bash_exists \
    || ! curl_exists \
    || ! jq_exists \
    || ! grep_exists \
    || ! coreutils_exists; then
    if [ -x /usr/bin/dnf5 ] \
        || [ -x /bin/dnf5 ] \
        || [ -x /sbin/dnf5 ]; then
        dnf5 -y install bash curl coreutils jq grep
    elif [ -x /usr/bin/dnf4 ] \
        || [ -x /bin/dnf4 ] \
        || [ -x /sbin/dnf4 ]; then
        dnf4 -y install bash curl coreutils jq grep
    elif [ -x /usr/bin/zypper ] \
        || [ -x /bin/zypper ] \
        || [ -x /sbin/zypper ]; then
        zypper --non-interactive install --auto-agree-with-licenses bash curl coreutils jq grep find
    elif [ -x /usr/bin/pacman ] \
        || [ -x /bin/pacman ] \
        || [ -x /sbin/pacman ]; then
        pacman --sync --noconfirm --refresh --sysupgrade bash curl coreutils jq grep
    elif [ -x /usr/bin/apt-get ] \
        || [ -x /bin/apt-get ] \
        || [ -x /sbin/apt-get ]; then
        apt-get update
        DEBIAN_FRONTEND=noninteractive apt-get -y install bash curl coreutils jq grep
    elif [ -x /usr/bin/apk ] \
        || [ -x /bin/apk ] \
        || [ -x /sbin/apk ]; then
        apk add --no-cache bash curl coreutils grep jq
    fi
fi
