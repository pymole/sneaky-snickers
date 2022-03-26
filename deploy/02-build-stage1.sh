#!/usr/bin/env bash

set -euo pipefail

FEATURES="seq ucb"

cargo +nightly build --release --target-dir=target/normal
env RUSTFLAGS="-Cprofile-generate=$PWD/target/pgo-data" cargo +nightly build --release --features "$FEATURES" --target-dir=target/withprofiler
