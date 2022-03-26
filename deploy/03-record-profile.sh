#!/usr/bin/env bash

set -euo pipefail

pushd arena/
./arena.py --config ../deploy/arena-config.jsonnet
popd

llvm-profdata merge -o ./target/merged.profdata ./target/pgo-data/
