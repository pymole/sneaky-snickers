#!/usr/bin/env bash

set -euo pipefail

FEATURES="seq ucb"
env RUSTFLAGS="-Cprofile-use=$PWD/target/merged.profdata" cargo +nightly build --release --features "$FEATURES" --target-dir=target/withpgo
