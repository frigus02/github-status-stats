#!/bin/bash
set -eu
while IFS= read -r line; do
    key=$(cut -d= -f1 <<<"$line")
    value=$(cut -d= -f2- <<<"$line")
    export "$key"="$value"
done <.env
"$@"
