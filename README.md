# xx network Substrate based blockchain node

### Rust Setup

First, complete the [basic Rust setup instructions](./doc/rust-setup.md).

### Build

Recommended OS to build the `xxnetwork-chain` binary is Ubuntu 20 or above.
The makefile provides build commands, of which the most important are:

```sh
make build-prod         # Build production ready node binary
make build-release      # Build release node binary
make build              # Build all packages
```

### MacOS users: setup to compile for Linux

Before being able to build for linux on macOS, the following extra steps are needed:

```sh
rustup target add x86_64-unknown-linux-gnu # Install linux GNU rust target
brew tap SergioBenitez/osxct               # Tap this project with brew
brew install x86_64-unknown-linux-gnu      # Install cross-compile tools for GNU
```

Then, the target for the rust compiler needs to be specified on any build command with `--target=x86_64-unknown-linux-gnu`.

### Test

The makefile provides the `test-pallets` command which runs unit tests for all custom pallets, as follows:

```sh
chainbridge
claims
staking
swap
xx-cmix
xx-economics
xx-team-custody
```

### Benchmarking

Included is a script that automatically runs the benchmarking code and calculates extrinsic weights for relevant pallets.

This can be executed with:

```sh
sh ./scripts/benchmark.sh
```

### Code Review

A code review was performed by ChainSafe. It focused mostly on the modifications to the Staking pallet
and the custom made xx network pallets for cmix, economics and team custody.
The report can be found [here](./doc/ChainSafe%20xxchain%20Code%20Review.pdf).
