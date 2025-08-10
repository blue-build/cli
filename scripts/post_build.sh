#!/usr/bin/env bash

set -euo pipefail
. /scripts/exports.sh

shopt -s nullglob
# needs nullglob, so that this array is empty if /opt is empty
optdirs=(/opt/*) # returns a list of directories in /opt
if [[ -n "${optdirs[*]}" ]]; then
    optfix_dir="/usr/lib/bluebuild-optfix"
    mkdir -pv "${optfix_dir}"
    echo "Creating symlinks to fix packages that installed to /opt:"
    for optdir in "${optdirs[@]}"; do
        opt=$(basename "${optdir}")
        lib_opt_dir="${optfix_dir}/${opt}"
        mv -v "${optdir}" "${lib_opt_dir}"
        echo "linking ${optdir} => ${lib_opt_dir}"
        echo "L+?  ${optdir}  -  -  -  -  ${lib_opt_dir}" | tee "/usr/lib/tmpfiles.d/bluebuild-optfix-${opt}.conf"
    done
fi

rm -rf /tmp/* /var/*

if feature_enabled "bootc" && command -v bootc >/dev/null; then
  bootc container lint
fi
