#!/bin/sh

set -eu

/scripts/setup.sh

optfix_dir="/usr/lib/opt"

echo "Preparing system for optfix..."
mkdir -pv "${optfix_dir}"

if [ -d /opt ] || [ -h /opt ]; then
    if  ls -A /opt/* 2>/dev/null; then
        echo "Moving all /opt/* into ${optfix_dir}"
        mv -v /opt/* "${optfix_dir}"
    fi
    rm -fr /opt
fi

echo "Linking /opt => ${optfix_dir}"
ln -fs "${optfix_dir}" /opt
