#!/bin/sh

if [ -f /etc/os-release ]; then
  export ID="$(cat /etc/os-release | grep -E '^ID=' | awk -F '=' '{print $2}')"

  if [ "$ID" = "alpine" ]; then
    echo "Setting up Alpine based image to run BlueBuild modules"
    apk update
    apk add --no-cache bash curl coreutils wget grep
  elif [ "$ID" = "ubuntu" ] || [ "$ID" = "debian" ]; then
    echo "Setting up Ubuntu based image to run BlueBuild modules"
    apt-get update
    apt-get install -y bash curl coreutils wget
  elif [ "$ID" = "fedora" ]; then
    echo "Settig up Fedora based image to run BlueBuild modules"
    dnf install -y --refresh bash curl wget coreutils
  else
    echo "OS not detected, exiting"
    exit 1
  fi
  cp /tmp/bins/yq /usr/bin/
else
  echo "File /etc/os-release not found, can't proceed"
  exit 1
fi
