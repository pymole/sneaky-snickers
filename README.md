Environment:
```
python3 -m venv venv
source venv/bin/activate

brew install libtorch

```

Install analysis dependencies
```
cd analysis
poetry install
```

In order to update package you need to bump version in pyproject.toml.
Alternetively you can update manually with maturin (provide LIBTORCH and LD_LIBRARY_PATH):
```commandline
maturin develop --release
```