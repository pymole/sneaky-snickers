local Bots = {
    binary(name, env={}, work_dir="..", exe="target/release/sneaky-snickers", mute=true): {
        name: name,
        type: "binary",
        work_dir: work_dir,
        exe: exe, // Relative to work_dir
        env: env,
        mute: mute,
    },

    from_commit(name, commit, env={}, mute=true): {
        name: name,
        type: "from_commit",
        build: {
            dir: "./build_dir/" + commit,
            // Should be tag or hash. Branch names are not properly implemented yet.
            commit: commit,
            flags: ["--release"]
        },
        run: {
            exe: "target/release/sneaky-snickers", // Relative to build.dir
            env: env,
            mute: mute,
        },
    },

    unmanaged(name, host="http://127.0.0.1", port="8000"): {
        name: name,
        type: "unmanaged",
        addresses: [host + ":" + port]
    }
};

local Settings = {
    search_with_time: { "MCTS_SEARCH_TIME": "200" },
    iterations: { "MCTS_ITERATIONS": "1000" },
};

local DefaultSetting = Settings.search_with_time;

{
    bots: [
        // Bots.from_commit("v1", "v1"),
        // Bots.from_commit("v2.8#t=200", "v2.8", { "MCTS_SEARCH_TIME": "200" }),

        // Bots.from_commit("v3.0", "v3.0", DefaultSetting),
        // Bots.from_commit("v3.2.1#cutoff=0", "v3.2.1", DefaultSetting { MCTS_ROLLOUT_CUTOFF: "0" }),
        // Bots.from_commit("v3.2.1#cutoff=3", "v3.2.1", DefaultSetting { MCTS_ROLLOUT_CUTOFF: "3" }),
        // Bots.from_commit("v3.3.1#cutoff=0", "v3.3.1", DefaultSetting { MCTS_ROLLOUT_CUTOFF: "0" }),
        // Bots.from_commit("v3.4#cutoff=0", "v3.4", DefaultSetting { MCTS_ROLLOUT_CUTOFF: "0" }),

        Bots.binary("master", DefaultSetting),

        // Bots.unmanaged("current"),
    ],

    ports: {
        from: 9000,
        to: 9100
    },

    rules: {
        build_dir: "./build_dir"
    },

    arena: {
        ratings_file: "./ratings.json",
        winrates_file: "./winrates.json",

        // Number of games to run.
        ladder_games: 100,

        // All games start with the same number of players. Precondition: number_of_players <= len(bots).
        number_of_players: 2,

        // How many games are run in parallel.
        // For maximum efficiency usually you want (number_of_players⋅parallel) ≈ cpu_count.
        parallel: 3,

        // Regulates sampling of players.
        // beta=0 — uniform.
        // beta=1 — proportional to rating.sigma.
        // beta>1 — prioritize players with high sigma even more.
        // Distribution is computed as p_i = σ_i^β / (∑_j σ_j^β).
        beta: 1.0,
    }
}
