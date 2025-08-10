#!/usr/bin/env bash

set -euo pipefail
. /scripts/exports.sh

optdirs=(/opt/*)
if [[ -n "${optdirs[*]}" ]]; then

  echo "Creating symlinks to fix packages that installed to /opt:"
  for optdir in "${optdirs[@]}"; do
    opt=$(basename "${optdir}")
    lib_opt_dir="/usr/lib/opt/${opt}"
    mv "${optdir}" "${lib_opt_dir}"
    mkdir -p "${lib_opt_dir}"
    tee "/usr/lib/tmpfiles.d/${opt}-bluebuild.conf" <<EOF
# create a link from ${optdir} to ${lib_opt_dir}
L  ${optdir}  -  -  -  -  ${lib_opt_dir}
EOF
  done
fi

rm -rf /tmp/* /var/*

if feature_enabled "bootc" && command -v bootc >/dev/null; then
  bootc container lint
fi
