#!/usr/bin/env bash

set -euo pipefail

source /tmp/scripts/exports.sh

# Function to print a centered text banner within a specified width
print_banner() {
  local term_width=80

  local text=" ${1} "        # Text to print
  local padding="$(printf '%0.1s' '='{1..600})"
  local padlen=0

  if (( ${#text} < term_width )); then
    padlen=$(( (term_width - ${#text}) / 2 ))
  fi

  printf '%*.*s%s%*.*s\n' 0 "$padlen" "$padding" "$text" 0 "$padlen" "$padding"
}

module="$1"
params="$2"
script_path="/tmp/modules/${module}/${module}.sh"

color_string "$(print_banner "Start '${module}' Module")" "33"
chmod +x ${script_path}

set +e
${script_path} "${params}"
RETVAL=$?
set -e

if [ $RETVAL ]; then
  color_string "$(print_banner  "End '${module}' Module")" "32"
else
  color_string "$(print_banner "Failed '${module}' Module")" "31"
  exit 1
fi
