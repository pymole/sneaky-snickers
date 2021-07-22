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
        Bots.from_commit('mcts_c#500#0.25', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "0.25",
        }),
        Bots.from_commit('mcts_c#500#0.35', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "0.35",
        }),
        Bots.from_commit('mcts_c#500#0.45', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "0.45",
        }),
        Bots.from_commit('mcts_c#500#0.5', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "0.5",
        }),
        Bots.from_commit('mcts_c#500#0.60', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "0.60",
        }),
        Bots.from_commit('mcts_c#500#0.75', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "0.75",
        }),
        Bots.from_commit('mcts_c#500#0.87', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "0.87",
        }),
        Bots.from_commit('mcts_c#500#1', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "1",
        }),
        Bots.from_commit('mcts_c#500#1.12', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "1.12",
        }),
        Bots.from_commit('mcts_c#500#1.25', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "1.25",
        }),
        Bots.from_commit('mcts_c#500#1.5', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "1.5",
        }),
        Bots.from_commit('mcts_c#500#1.75', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "1.75",
        }),
        Bots.from_commit('mcts_c#500#2', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "500",
            "MCTS_C": "2",
        }),
        Bots.from_commit('mcts_c#1000#0.25', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "0.25",
        }),
        Bots.from_commit('mcts_c#1000#0.35', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "0.35",
        }),
        Bots.from_commit('mcts_c#1000#0.45', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "0.45",
        }),
        Bots.from_commit('mcts_c#1000#0.5', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "0.5",
        }),
        Bots.from_commit('mcts_c#1000#0.60', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "0.60",
        }),
        Bots.from_commit('mcts_c#1000#0.75', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "0.75",
        }),
        Bots.from_commit('mcts_c#1000#0.87', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "0.87",
        }),
        Bots.from_commit('mcts_c#1000#1', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "1",
        }),
        Bots.from_commit('mcts_c#1000#1.12', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "1.12",
        }),
        Bots.from_commit('mcts_c#1000#1.25', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "1.25",
        }),
        Bots.from_commit('mcts_c#1000#1.5', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "1.5",
        }),
        Bots.from_commit('mcts_c#1000#1.75', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "1.75",
        }),
        Bots.from_commit('mcts_c#1000#2', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "1000",
            "MCTS_C": "2",
        }),
        Bots.from_commit('mcts_c#2000#0.25', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "0.25",
        }),
        Bots.from_commit('mcts_c#2000#0.35', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "0.35",
        }),
        Bots.from_commit('mcts_c#2000#0.45', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "0.45",
        }),
        Bots.from_commit('mcts_c#2000#0.5', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "0.5",
        }),
        Bots.from_commit('mcts_c#2000#0.60', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "0.60",
        }),
        Bots.from_commit('mcts_c#2000#0.75', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "0.75",
        }),
        Bots.from_commit('mcts_c#2000#0.87', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "0.87",
        }),
        Bots.from_commit('mcts_c#2000#1', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "1",
        }),
        Bots.from_commit('mcts_c#2000#1.12', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "1.12",
        }),
        Bots.from_commit('mcts_c#2000#1.25', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "1.25",
        }),
        Bots.from_commit('mcts_c#2000#1.5', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "1.5",
        }),
        Bots.from_commit('mcts_c#2000#1.75', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "1.75",
        }),
        Bots.from_commit('mcts_c#2000#2', 'c14d36f6f5efab73d042d99dcf74501191535bc7', {
            "MCTS_ITERATIONS": "2000",
            "MCTS_C": "2",
        }),
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
        ladder_games: 500,

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
