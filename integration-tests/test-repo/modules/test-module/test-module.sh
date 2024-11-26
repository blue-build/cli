#!/bin/bash

set -euo pipefail

echo "This is a test module"

get_json_array FILES '.test[]' '{"test":[1,2,3]}'

echo "${FILES[@]}"
