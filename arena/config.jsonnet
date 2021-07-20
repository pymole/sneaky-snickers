{
    bots: [
        {
            name: "v1",
            type: "from_commit",
            build: {
                dir: "./build_dir/v1",
                commit: "v1", // Should be tag or hash. Branch names are not properly implemented yet.
                flags: ["--release"]
            },
            run: {
                exe: "target/release/sneaky-snickers", // Relative to build.dir
                env: {},
                mute: true,
            },
        },
        {
            name: "v2.1#10",
            type: "from_commit",
            build: {
                dir: "./build_dir/v2",
                commit: "v2.1",
                flags: ["--release"]
            },
            run: {
                exe: "target/release/sneaky-snickers",
                env: { "MCTS_ITERATIONS": "10" },
                mute: true,
            },
        },
        {
            name: "v2.1#50",
            type: "from_commit",
            build: {
                dir: "./build_dir/v2",
                commit: "v2.1",
                flags: ["--release"]
            },
            run: {
                exe: "target/release/sneaky-snickers",
                env: { "MCTS_ITERATIONS": "50" },
                mute: true,
            },
        },
        {
            name: "v2.1#100",
            type: "from_commit",
            build: {
                dir: "./build_dir/v2",
                commit: "v2.1",
                flags: ["--release"]
            },
            run: {
                exe: "target/release/sneaky-snickers",
                env: { "MCTS_ITERATIONS": "100" },
                mute: true,
            },
        },
        {
            name: "v2.1#500",
            type: "from_commit",
            build: {
                dir: "./build_dir/v2",
                commit: "v2.1",
                flags: ["--release"]
            },
            run: {
                exe: "target/release/sneaky-snickers",
                env: { "MCTS_ITERATIONS": "500" },
                mute: true,
            },
        },
        {
            name: "v2.1#1000",
            type: "from_commit",
            build: {
                dir: "./build_dir/v2",
                commit: "v2.1",
                flags: ["--release"]
            },
            run: {
                exe: "target/release/sneaky-snickers",
                env: { "MCTS_ITERATIONS": "1000" },
                mute: true,
            },
        },
        // {
        //     name: "current",
        //     type: "unmanaged",
        //     addresses: ["http://127.0.0.1:8000"]
        // }
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
        ladder_games: 100, // number of games to run
        number_of_players: 2, // per game
    }
}
