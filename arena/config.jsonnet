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
        ratings_file: "./ratings_4.json",
        ladder_games: 300, // number of games to run
        number_of_players: 4, // per game
        parallel: 2,
    }
}
