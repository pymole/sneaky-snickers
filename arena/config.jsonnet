local Bots = {
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

    current(name="current", host="http://127.0.0.1", port="8000"): {
        name: name,
        type: "unmanaged",
        addresses: [host + ":" + port]
    }
};

{
    bots: [
        Bots.from_commit("v1", "v1"),
        Bots.from_commit("v2.1#500", "v2.1", { "MCTS_ITERATIONS": "500" }),
        Bots.from_commit("v2.1#1000", "v2.1", { "MCTS_ITERATIONS": "1000" }),
        Bots.from_commit("v2.2#500", "v2.2", { "MCTS_ITERATIONS": "500" }),
        Bots.from_commit("v2.2#1000", "v2.2", { "MCTS_ITERATIONS": "1000" }),
        Bots.from_commit("v2.3#500", "v2.3", { "MCTS_ITERATIONS": "500" }),
        // Bots.current(),
    ],

    ports: {
        from: 9000,
        to: 9100
    },

    rules: {
        build_dir: "./build_dir"
    },

    arena: {
        ratings_file: "./ratings_test.json",

        // Number of games to run.
        ladder_games: 300,

        // All games start with the same number of players. Precondition: number_of_players <= len(bots).
        number_of_players: 2,

        // How many games are run in parallel.
        // For maximum efficiency usually you want (number_of_players⋅parallel) ≈ cpu_count.
        parallel: 5,

        // Regulates sampling of players.
        // beta=0 — uniform.
        // beta=1 — proportional to rating.sigma.
        // beta>1 — prioritize players with high sigma even more.
        // Distribution is computed as p_i = σ_i^β / (∑_j σ_j^β).
        beta: 1.5,
    }
}
