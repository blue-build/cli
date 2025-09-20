#!/bin/sh

if [ -f /etc/os-release ]; then
  # Initialize variable to store the ID
  ID=""

  # Read the /etc/os-release file line by line
  while IFS== read -r key value; do
    # Check if the key is 'ID'
    if [ "$key" = "ID" ]; then
      # Remove any quotes from the value and store it in id variable
      ID=$(echo "$value" | tr -d '"')
      break
    fi
  done < /etc/os-release

  if [ "$ID" = "alpine" ]; then
    echo "Setting up Alpine based image to run BlueBuild modules"
    apk update
    apk add --no-cache bash curl coreutils wget grep jq
  elif [ "$ID" = "ubuntu" ] || [ "$ID" = "debian" ]; then
    echo "Setting up Ubuntu based image to run BlueBuild modules"
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y bash curl coreutils wget jq
  elif [ "$ID" = "fedora" ]; then
    echo "Settig up Fedora based image to run BlueBuild modules"
    dnf install -y --refresh bash curl wget coreutils jq
  else
    echo "OS not detected, proceeding without setup"
  fi
fi
