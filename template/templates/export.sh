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
    printf "Usage: get_yaml_array VARIABLE_TO_STORE_RESULTS JQ_QUERY MODULE_CONFIG\n" >&2
    return 1
  fi

  readarray -t arr < <(echo "$module_config" | yq -r "$jq_query")
}

# Parse OS version and export it
export OS_VERSION=$(grep -Po "(?<=VERSION_ID=)\d+" /usr/lib/os-release)

# Export functions for use in sub-shells or sourced scripts
export -f get_yaml_array

