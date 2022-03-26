#!/usr/bin/env bash

set -euo pipefail

pushd arena/
python3.9 ./arena.py --config ../deploy/arena-config.jsonnet
popd

llvmprofdata=$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata
$llvmprofdata merge -o ./target/merged.profdata ./target/pgo-data/

