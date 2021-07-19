{
    bots: {
        v1: {
            type: "from_commit",
            build: {
                dir: "./build_dir/v1",
                commit: "v1",
                flags: ["--release"]
            },
            mute: true,
            launch: "target/release/sneaky-snickers"
        },
        current: {
            type: "unmanaged",
            addresses: ["http://127.0.0.1:8000"]
        }
    },

    ports: {
        from: 9000,
        to: 9100
    },

    rules: {
        build_dir: "./build_dir"
    },

    arena: {
        ranks_file: "./ranks.json",
        ladder_games: 10,
        number_of_players: 2,
    }
}
