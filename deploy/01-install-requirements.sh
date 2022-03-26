#!/usr/bin/bash

set -euo pipefail

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup install nightly
rustup +nightly component add llvm-tools-preview
sudo apt install clang golang-go python3.9-venv python3-pip python3.9-dev
python3.9 -mpip install -r arena/requirements.txt --user
