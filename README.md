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

You'll find the compiled artifact under `artifacts/goblin_vesting.wasm`

## Contract Life-Cycle

After Instantiation the Contract is completely inactive. The admin can send the `KickOff` message to put the contract into a dormant state. In this stage the contract starts the lock-up period where no funds of the CW20 balance are paid out to the board members. The `KickOff` message contains the UNIX timestamp of when the linear payout schedule should start.

```
{
  "date": "<u64>",
}
```

When the contract reaches that date it automatically changes into the active state. From there on board members accrue an unlock amount on a regular basis. The payout is linear in the time domain and weighted according to the weight properties of the corresponding members. Below you'll find a schematic for the contract lifecycle.

![life_cycle](https://github.com/luncgoblins/goblin-vesting/assets/29800180/70746115-06c6-449d-a73b-afd07453fbc1)

After contract expiry most of the functions are deactivated. Members may be still eligibile to withdraw the remaining unlock that they have accrued (but not withdrawn) during the active period

## Instantiate

For instantiation send a `MsgInstantiateContract` to the blockchain with the following json msg:

``` 
{

  // amount of seconds you want the linear payout schedule to last
  "vesting_period": "<u64>",

  // vesting cw20 token address
  "token": "<string>",

  // list of initial board members
  shareholders:[
    {
      "address": "<string>"
      "weight": "<u64>"
    },
    {
      "address": "<string>"
      "weight": "<u64>"
    },
    ...
    {
      "address": "<string>"
      "weight": "<u64>"
    }
  ],

  // address for permissioned access
  "admin": "<string>",

}
```

## Supported Queries

The contract gives you the following query endpoints:

  - Query current contract configuration
  - Query current board member info
  - Query current eligible unlocks per member
