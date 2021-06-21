#!/usr/bin/env bash

set -euo pipefail

SCRIPT_PATH=$(dirname "${BASH_SOURCE[0]}")
SLEEP_INTERVAL=5m

while true; do
    $SCRIPT_PATH/scrape_games.py "$@"
    echo "Sleeping for $SLEEP_INTERVAL"
    sleep $SLEEP_INTERVAL
done
