#!/usr/bin/env bash

set -euo pipefail

source /tmp/exports.sh

# Function to print a centered text banner within a specified width
print_banner() {
  local term_width=120

  local text=" ${1} "        # Text to print
  local padding="$(printf '%0.1s' '='{1..600})"
  local padlen=0

  if (( ${#text} < term_width )); then
    padlen=$(( (term_width - ${#text}) / 2 ))
  fi

  printf '%*.*s%s%*.*s\n' 0 "$padlen" "$padding" "$text" 0 "$padlen" "$padding"
}

title_case() {
  echo $(tr '[:lower:]' '[:upper:]' <<< ${1:0:1})${1:1}
}

module="$1"
params="$2"
script_path="/tmp/modules/${module}/${module}.sh"

print_banner "Start $(title_case ${module}) Module"
chmod +x ${script_path}
${script_path} "${params}"
print_banner "End $(title_case ${module}) Module"
