#!/usr/bin/env bash

export ROCKET_ADDRESS=0.0.0.0
export ROCKET_PORT=8000

export MCTS_SEARCH_TIME=400
# export MCTS_TABLE_CAPACITY=
# export MCTS_DRAW_REWARD=
# export MCTS_SELECT_DEPTH=
# export MCTS_WORKERS=
# export MCTS_ROLLOUT_CUTOFF=

exec target/withpgo/release/sneaky-snickers
