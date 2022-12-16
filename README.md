Environment:
```
python3 -m venv venv
source venv/bin/activate

sudo apt-get install ubuntu-drivers-common
sudo ubuntu-drivers devices
sudo apt-get install nvidia-384
sudo nvidia-smi

export LIBTORCH=~/battlesnake/libtorch
export LD_LIBRARY_PATH=~/battlesnake/libtorch

curl https://repo.anaconda.com/archive/Anaconda3-2021.11-Linux-x86_64.sh --output anaconda.sh
```

In order to update package you need to bump version in pyproject.toml.
Alternetively you can update manually with maturin (provide LIBTORCH and LD_LIBRARY_PATH):
```commandline
maturin develop --release
```