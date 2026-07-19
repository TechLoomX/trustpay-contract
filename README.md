# trustpay-contract

Milestone-based crypto escrow built on Stellar.

A Soroban smart contract that holds funds in escrow per milestone, releasing
them only when the client approves. All state transitions (funding,
submission, approval, disputes, refunds, cancellation) are enforced on-chain,
so neither party can unilaterally alter an agreement once funds are
deposited.

## How it works

- A `client` creates an escrow for a `freelancer`, split into one or more
  milestones, each with an amount and a hash of its (off-chain) description.
- The client deposits funds for a milestone into the contract.
- The freelancer submits the milestone as complete.
- The client approves, and funds move from the contract to the freelancer.
- Either party can raise a dispute on a funded/submitted milestone, freezing
  it pending manual/off-chain resolution (on-chain arbitration is a future
  item, not part of v1).
- The client can refund an unsubmitted milestone, or cancel the whole escrow
  (auto-refunding any funded-but-not-yet-submitted milestones).

See [`contracts/escrow/src/lib.rs`](contracts/escrow/src/lib.rs) for the full
function reference and [`contracts/escrow/src/types.rs`](contracts/escrow/src/types.rs)
for the data model.

## Requirements

- Rust with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli/stellar-cli) (`stellar`), used for building and deploying

## Build

```sh
make build
```

Produces `target/wasm32v1-none/release/escrow.wasm`.

## Test

```sh
make test
```

Runs the contract's unit tests (`contracts/escrow/src/test.rs`) covering the
happy path, multi-milestone completion, refunds, auth failures, invalid state
transitions, dispute freezes, and cancellation with mixed milestone states.

## Deploy to testnet

```sh
make deploy-testnet
```

Generates (or reuses) a funded testnet identity named `trustpay-deployer`,
builds the contract, and deploys it to Stellar testnet. Copy the printed
contract ID into the placeholder below.

**Testnet contract ID:** `TBD`

## Repo layout

```text
trustpay-contract/
  contracts/
    escrow/
      src/
        lib.rs       # public contract functions
        types.rs     # Escrow, Milestone, DataKey, enums
        errors.rs     # Error enum
        events.rs     # contract events
        test.rs       # unit tests
      Cargo.toml
  Makefile            # build, test, deploy-testnet targets
  README.md
```
