get_yaml_array() { 
  set -euo pipefail
  readarray -t "$1" < <(echo "$3" | yq -I=0 "$2")
} 

rmex() {
  set -euo pipefail
  set -x
  local search_dir="$1"
  shift

  local exclude_patterns=("$@")

  local grep_pattern=$(printf "|^%s" "${exclude_patterns[@]}")
  grep_pattern="${grep_pattern:1}"

  find "$search_dir" -type f | grep -Ev "($grep_pattern)" | xargs rm -f
  set +x
}

export -f get_yaml_array
export -f rmex
export OS_VERSION=$(grep -Po '(?<=VERSION_ID=)\d+' /usr/lib/os-release)
