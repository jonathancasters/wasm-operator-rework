# Hello world example
This example illustrates that we can run a child wasm module in the parent's runtime (up to [this commit](https://github.com/jonathancasters/wasm-operator-rework/commit/8e4d08deefa6a62323def91add984d7f927c0cee))
## Run code
You can run this example by running the following command from this directory:
```sh
cargo run --manifest-path ../../parent/Cargo.toml ./helloworld.yaml
```
or you could execute the `run.sh` script in this folder:
```sh
chmod +x ./run.sh
./run.sh
```
