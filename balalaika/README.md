# Version History

Only changes in agent playing ability are mentioned here.

- `v1` — Greedy snake, that goes after closest food.
- `v2` — MCTS.
- `v2.1` — Bugfix for the case when there are no available moves.
- `v2.2` — Bugfix in game engine for head-to-head collisions.
- `v2.3`
    - Bugfix for safe zone calculation.
    - Bugfix: do not expand nodes in terminal states.
- `v2.4` — Set UCB "explore" constant to с=0.6.
- `v2.4.timed` — Support limiting search by time.
- `v2.5` — Improve board's hash calculation speed.
- `v2.6` — Shrink safe zone in simulation.
- `v2.7` — New rollout heuristic.
- `v2.8` — UCB1-tuned.
- `v2.8.stats` — Print UCB stats for root node.
- `v3.0`
    - Drop support for royale mode.
    - Initial support for wrapped+spiral mode.
    - Reserve
- `v3.1` (note: has a bug in >2 player games)
    - 75 Wins, 10 Draws, 15 Losses against `v3.0` (in single-threaded mode)
    - update logic of eating food in hazard zone
    - zobrist hash
    - optimize code: reduce memory allocations, cache q value
    - generate food during mcts
    - prefer being longer instead of being healthier in reward
    - implement multi-threaded mode
- `v3.2`
    - Bugfix: snake used to work incorrectly in >2 player games and would crash sometimes
    - improvemed performance for multi-threaded mode
- `v3.2.1` — unrestricted MCTS_ROLLOUT_CUTOFF
- `v3.3` — New flood fill reward heuiristic
- `v3.3.1` — unrestricted MCTS_ROLLOUT_CUTOFF
- `v3.4`
    - Improved heuristic: flood_fill% * length%
    - Approx. 8x speedup of flood fill procedure
- `v3.5`
    - Version that played in Spring League 2022 tournament and reached finals (--features="par ucb" with default
      settings (search_time 300ms) on 32 vCPU machine in Yandex Cloud).
    - Set default MCTS_ROLLOUT_CUTOFF=0
    - Optimize parallel mcts
    - (possibly) Optimize engine interface
    - Use flood fill in parallel mcts
    - Recolor to #dd3333
    - Change default draw_reward to 0.01
