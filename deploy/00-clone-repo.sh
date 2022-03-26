#!/usr/bin/bash

# If this repository is not cloned yet on remote machine, just copy following commands in ssh terminal.

sudo apt update
sudo apt install git
echo "To fetch/push changes to git server you need to run ssh with `-A` flag."
echo "If following command fails, relogin in ssh with `ssh -A username@hostname`"
git clone git@github.com:pymole/sneaky-snickers.git
cd sneaky-snickers
