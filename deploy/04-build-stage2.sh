#!/usr/bin/env bash

set -euo pipefail

FEATURES="par ucb"
env RUSTFLAGS="-Cprofile-use=$PWD/target/merged.profdata" cargo +nightly build --release --features "$FEATURES" --target-dir=target/withpgo
