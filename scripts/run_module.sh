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

get_script_path() {
  local script_name="$1"
  local extensions=("nu" "sh" "bash")
  local base_script_path="/tmp/modules/${script_name}/${script_name}"
  local tried_scripts=()

  # See if
  if [[ -f "${base_script_path}" ]]; then
    echo "${base_script_path}"
    return 0
  fi
  tried_scripts+=("${script_name}")

  # Iterate through each extension and check if the file exists
  for ext in "${extensions[@]}"; do
    local script_path="${base_script_path}.${ext}"
    tried_scripts+=("${script_name}.${ext}")
      
    if [[ -f "$script_path" ]]; then
      # Output only the script path without extra information
      echo "$script_path"
      return 0  # Exit the function when the first matching file is found
    fi
  done

  # If no matching file was found
  echo "Failed to find scripts matching: ${tried_scripts[*]}" >&2
  return 1
}

module="$1"
params="$2"
script_path="$(get_script_path "$module")"
nushell_version="$(echo "${params}" | jq '.["nushell-version"] // empty')"

export PATH="/usr/libexec/bluebuild/nu/:$PATH"

color_string "$(print_banner "Start '${module}' Module")" "33"
chmod +x "${script_path}"

if "${script_path}" "${params}"; then
  color_string "$(print_banner  "End '${module}' Module")" "32"

else
  color_string "$(print_banner "Failed '${module}' Module")" "31"
  exit 1
fi
