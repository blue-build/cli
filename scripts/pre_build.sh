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

# Set fixed modification times for preinstalled binaries for layer reproducibility
for file in /usr/bin/bluebuild /usr/bin/cosign /usr/libexec/bluebuild/nu; do
    if [ -d "${file}" ]; then
        find "${file}" -xdev -exec touch -cm -d '1970-01-01T00:00:00Z' '{}' +
    else
        touch -cm -d '1970-01-01T00:00:00Z' "${file}"
    fi
done

echo "Linking /opt => ${optfix_dir}"
ln -fs "${optfix_dir}" /opt
