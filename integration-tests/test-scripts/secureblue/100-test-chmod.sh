#!/usr/bin/env bash

set -euo pipefail

# Function to check if hardening has been applied correctly
check_hardening() {
  local sysctl_conf="/usr/etc/sysctl.d/hardening.conf"
  local bwrap="/usr/bin/bwrap"

  # Check for the presence of user namespace hardening
  if grep -q "user.max_user_namespaces = 0" "$sysctl_conf" &&
     grep -q "kernel.unprivileged_userns_clone = 0" "$sysctl_conf"; then
    printf "Hardening settings are correctly applied in %s\n" "$sysctl_conf"
  else
    printf "Hardening settings are missing or incorrect in %s\n" "$sysctl_conf" >&2
    return 1
  fi

  # Check ownership and SUID bit of bwrap
  if [ "$(stat -c '%U' "$bwrap")" = "root" ] && [ "$(stat -c '%a' "$bwrap")" -eq 4755 ]; then
    printf "%s ownership and permissions are correctly set\n" "$bwrap"
  else
    printf "%s ownership or permissions are incorrect\n" "$bwrap" >&2
    return 1
  fi
}

# Main function to orchestrate the checks
main() {
  set -euo pipefail
  
  # Perform the checks
  if ! check_hardening; then
    printf "Hardening checks failed\n" >&2
    exit 1
  else
    printf "All hardening checks passed\n"
  fi
}

main "$@"
