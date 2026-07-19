NETWORK ?= testnet
SOURCE_ACCOUNT ?= trustpay-deployer
WASM := target/wasm32v1-none/release/escrow.wasm

.PHONY: build test deploy-testnet clean

build:
	stellar contract build

test:
	cargo test

# Generates (or reuses) a funded testnet identity, then deploys the built
# wasm and prints the contract ID to save in README.md.
deploy-testnet: build
	stellar keys generate $(SOURCE_ACCOUNT) --network $(NETWORK) --fund || true
	stellar contract deploy \
		--wasm $(WASM) \
		--source-account $(SOURCE_ACCOUNT) \
		--network $(NETWORK)

clean:
	cargo clean
