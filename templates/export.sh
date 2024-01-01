#!/usr/bin/env bash

get_yaml_array() { 
  readarray "$1" < <(echo "$3" | yq -I=0 "$2")
} 

export -f get_yaml_array
export OS_VERSION=$(grep -Po '(?<=VERSION_ID=)\d+' /usr/lib/os-release)
