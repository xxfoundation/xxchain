#######################
### xx network only ###
#######################

# Builds production ready xxnetwork-chain binary containing only xxnetwork runtime
build-prod:
	@cargo build -p xxnetwork-cli --profile production

# Builds xxnetwork-chain binary containing only xxnetwork runtime
build-release:
	@cargo build -p xxnetwork-cli --release

#######################
###  xx canary only ###
#######################

# Builds production ready xxnetwork-chain binary containing only canary runtime
build-canary-prod:
	@cargo build -p xxnetwork-cli --profile production --no-default-features --features cli,canary

# Builds xxnetwork-chain binary containing only canary runtime
build-canary-release:
	@cargo build -p xxnetwork-cli --release --no-default-features --features cli,canary

#######################
###  both runtimes  ###
#######################

# Builds all packages
build:
	@cargo build --release --features canary

# Builds all packages with accelerated runtimes
build-dev:
	@cargo build --release --features fast-runtime

# Builds xxnetwork-chain binary with runtime-benchmarks feature enabled
# Uses production profile, in order to correctly benchmark the production binary
build-bench:
	@cargo build -p xxnetwork-cli --profile production --features runtime-benchmarks

# Builds xxnetwork-chain binary with try-runtime feature enabled
build-try-runtime:
	@cargo build -p xxnetwork-cli --release --features try-runtime

# Builds xxnetwork-chain binary containing accelerated runtimes with the try-runtime feature enabled
build-dev-try-runtime:
	@cargo build -p xxnetwork-cli --release --features fast-runtime,try-runtime

#######################
###  build runtimes ###
#######################

build-canary-runtime:
	@SRTOOL_TAG="1.66.1" srtool build --package canary-runtime

build-xxnetwork-runtime:
	@SRTOOL_TAG="1.66.1" srtool build --package xxnetwork-runtime

build-runtimes: build-canary-runtime build-xxnetwork-runtime

#######################
###      tests      ###
#######################

all-tests:
	@echo "Running all unit tests\n"
	@cargo test --workspace

test-pallets:
	@echo "Running unit tests for all pallets\n"
	@cd chainbridge; cargo test; cd ../
	@cd claims; cargo test; cd ../
	@cd swap; cargo test; cd ../
	@cd xx-betanet-rewards; cargo test; cd ../
	@cd xx-cmix; cargo test; cd ../
	@cd xx-economics; cargo test; cd ../
	@cd xx-public; cargo test; cd ../
	@cd xx-team-custody; cargo test; cd ../
