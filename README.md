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