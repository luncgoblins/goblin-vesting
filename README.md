## Open Source Vesting Contract

This is an open source vesting contract for CW20 tokens on Terra Classic. It has the following features:

- Linear payout within predefined period
- Weighted board members who are eligibile for payout according to their weights
- Dormant (or cliff) period before linear payout schedule
- Block time resolution payout calculation
- Adjustable board
- Leaving board members can receive severence pay

## Build

Follow this guide (https://book.cosmwasm.com/setting-up-env.html) to set up your build environment. If you have not already, ou may also want to install `run-script` extension for cargo:

```
$ cargo install cargo-run-script
```

To build the contract, run:

```
$ cargo build
$ cargo run-script optimize
```

You'll find the compiled artifact under `artifacts/goblin_vesting.wasm

## Instantiate

For instantiation send a `MsgInstantiateContract` with the following json msg:

{
	vesting_period: u64,
	token: String,
	shareholders: Vec<InitialShareholder>,
	admin: String,
}

## Build
