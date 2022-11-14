Environment:
```
python3 -m venv venv
source venv/bin/activate
```

Install analysis dependencies
```
cd analysis
poetry install
```

In order to update package you need to bump version in pyproject.toml.
Alternetively you can update manually with maturin:
```commandline
maturin develop --release
```

Selfplay:
```commandline
cd balalaika
MCTS_ITERATIONS=5000 MONGO_URI='mongodb://battlesnake:battlesnake@localhost:27017/battlesnake?authSource=admin' cargo run --bin selfplay --release
```
