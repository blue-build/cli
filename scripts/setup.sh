#!/bin/sh

if [ -f /etc/os-release ]; then
  export ID="$(awk -F= '/^ID=/ {gsub(/"/, "", $2); print $2}' /etc/os-release)"

  if [ "$ID" = "alpine" ]; then
    echo "Setting up Alpine based image to run BlueBuild modules"
    apk update
    apk add --no-cache bash curl coreutils wget grep jq
  elif [ "$ID" = "ubuntu" ] || [ "$ID" = "debian" ]; then
    echo "Setting up Ubuntu based image to run BlueBuild modules"
    apt-get update
    apt-get install -y bash curl coreutils wget jq
  elif [ "$ID" = "fedora" ]; then
    echo "Settig up Fedora based image to run BlueBuild modules"
    dnf install -y --refresh bash curl wget coreutils jq
  else
    echo "OS not detected, proceeding without setup"
  fi
fi
