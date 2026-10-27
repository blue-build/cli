#!/usr/bin/env bash

set -euo pipefail
. /scripts/exports.sh

shopt -s nullglob

optfix_dir="/usr/lib/opt"
# needs nullglob, so that this array is empty if /opt is empty
optdirs=("${optfix_dir}"/*) # returns a list of directories in /opt
if [[ -n "${optdirs[*]}" ]]; then
    echo "Creating symlinks to fix packages that installed to /opt:"
    for optdir in "${optdirs[@]}"; do
        opt=$(basename "${optdir}")
        lib_opt_dir="${optfix_dir}/${opt}"
        link_opt_dir="/opt/${opt}"
        echo "Linking ${link_opt_dir} => ${lib_opt_dir}"
        echo "L+?  \"${link_opt_dir}\"  -  -  -  -  ${lib_opt_dir}" | tee "/usr/lib/tmpfiles.d/99-bluebuild-optfix-${opt}.conf"
    done
fi

rm -rf /tmp/* /var/* /opt
ln -fs /var/opt /opt

# Relink rpm-ostree-base-db to rpmdb to ensure it correctly reflects the system
# image's rpmdb and doesn't carry over package info from the base image.
# See: https://github.com/coreos/rpm-ostree/issues/4554
for file in rpmdb.sqlite rpmdb.sqlite-shm rpmdb.sqlite-wal; do
    target="/usr/share/rpm/${file}"
    link_path="/usr/lib/sysimage/rpm-ostree-base-db/${file}"
    if [[ -f "${target}" && -f "${link_path}" ]]; then
        # Note, this needs to be a hardlink, not a symbolic link.
        ln -f "${target}" "${link_path}"
    fi
done

# if feature_enabled "bootc" && command -v bootc > /dev/null; then
#   bootc container lint
# fi
