#!/usr/bin/env bash

# Function to retrieve module configs and populate an array
# Arguments:
#   1. Variable name to store result
#   2. jq query
#   3. Module config content
get_yaml_array() {
  local -n arr=$1
  local jq_query=$2
  local module_config=$3

  if [[ -z $jq_query || -z $module_config ]]; then
    echo "Usage: get_yaml_array VARIABLE_TO_STORE_RESULTS JQ_QUERY MODULE_CONFIG" >&2
    return 1
  fi

  readarray -t arr < <(echo "$module_config" | yq -I=0 "$jq_query")
}

# Function to retrieve module configs and populate an array
# Arguments:
#   1. Variable name to store result
#   2. jq query
#   3. Module config content
get_json_array() {
  local -n arr=$1
  local jq_query=$2
  local module_config=$3

  if [[ -z $jq_query || -z $module_config ]]; then
    echo "Usage: get_json_array VARIABLE_TO_STORE_RESULTS JQ_QUERY MODULE_CONFIG" >&2
    return 1
  fi

  readarray -t arr < <(echo "$module_config" | jq -r "$jq_query")
}

color_string() {
  local string="$1"
  local color_code="$2"
  local reset_code="\033[0m"

  # ANSI color codes: https://en.wikipedia.org/wiki/ANSI_escape_code#Colors
  # Example color codes: 31=red, 32=green, 33=yellow, 34=blue, 35=magenta, 36=cyan, 37=white

  # Check if color code is provided, otherwise default to white (37)
  if [[ -z "$color_code" ]]; then
    color_code="37"
  fi

  # Determine if we should force color
  if [ -n "${FORCE_COLOR:-}" ] || [ -n "${CLICOLOR_FORCE:-}" ]; then
    # Force color: Apply color codes regardless of whether output is a TTY
    echo -e "\033[${color_code}m${string}${reset_code}"
  elif [ -t 1 ]; then
    # Output is a TTY and color is not forced: Apply color codes
    echo -e "\033[${color_code}m${string}${reset_code}"
  else
    # Output is not a TTY: Do not apply color codes
    echo "$string"
  fi
}

# Parse OS version and export it
export OS_VERSION=$(grep -Po "(?<=VERSION_ID=)\d+" /usr/lib/os-release)
export OS_ARCH=$(uname -m)

# Export functions for use in sub-shells or sourced scripts
export -f get_yaml_array
export -f get_json_array

mkdir -p /var/roothome /var/opt /var/lib/alternatives /var/opt /var/usrlocal
