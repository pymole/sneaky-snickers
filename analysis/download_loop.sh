#!/usr/bin/env bash

set -euo pipefail

SCRIPT_PATH=$(dirname "${BASH_SOURCE[0]}")
SLEEP_INTERVAL=5m

while true; do
    # Uncomment `|| true` to ignore errors.
    $SCRIPT_PATH/scrape.py recent "$@" # || true
    echo "Sleeping for $SLEEP_INTERVAL"
    sleep $SLEEP_INTERVAL
done
