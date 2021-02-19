# ledger.rs

The `ledger.rs` library implements all API commands of the Ledger Nano S/X app and provides some abstraction for common tasks like generating addresses or signing essences.

It is free of dependencies to other iota specific libraries (like `iota.rs` or `bee`).

The API specification can be found here: [API-Specification](https://github.com/iotaledger/ledger-iota-app/blob/develop/docs/specification_chrysalis.md)

## Example

Following an example how a ledger object is instanciated and an address is generated that is converted into bech32 representation:

```rust
const HARDENED : u32 = 0x80000000;

// bip32 path follows: 2c'/107a'/account'/change'/index'
let mut ledger = iota_ledger::get_ledger_by_type(0 | HARDENED, TransportTypes::TCP, None)?;

let input_bip32_index = LedgerBIP32Index {
    bip32_index: 1 | HARDENED,
    bip32_change: 0 | HARDENED,
};

// get one single address, don't show it on the UI
let input_addr_bytes: [u8; 32] = ledger
    .get_addresses(false, input_bip32_index, 1)
    .expect("error get new address")
    .first()
    .unwrap();

// add the address_type byte
let mut addr_bytes_with_type = [0u8; 33];
addr_bytes_with_type[0] = 0; // ed25519 address_type
addr_bytes_with_type[1..33].clone_from_slice(&input_addr_bytes[..]);

// convert the 33 byte address into a bech32 string
let bech32_address = bech32::encode(hrp, 
    addr_bytes_with_type.to_base32()).unwrap();

// output the address
println!("{}", bech32_address);

```



# Test Program `cli.rs`

There is a test program that can be used for automatic testing of the app running in the Speculos simulator (Nano S and Nano X) or on a real device.

In comparison to `ledger.rs`, the `cli.rs` test program has dependencies to `bee`.

## What it does

The program builds (pseudo-)random essences of messages for several different account-, input-, output-, remainder-configurations.

The number of runs is pseudo-random. Currently, a total of 10000 pseudo-random configurations are generated, but only unique configurations are tested - the rest is skipped. In this case the total number of runs is 62.

Currently, the number of "valid configurations" is limited by the `MAX_SUM_INPUTS_OUTPUTS` that is 16. That means, essences up to a total count of 16 (sum of the count of inputs + outputs) with and without extra remainder are generated.

The test program also can record APDU transfers as `bin`, `hex` or `json` for automatic testing with a [dependency-free C testing program](https://github.com/iotaledger/ledger-iota-app/tree/develop/tests).

Additionally, interactive or non-interactive tests can be done. The difference is that in non-interactive mode no user interaction is required to sign essences. Non-interactive mode only is available if the [app](https://github.com/iotaledger/ledger-iota-app) is compiled with `DEBUG=1` or `SPECULOS=1` flag. (The Debug flag also changes the bip32-path to `2c'/1'/account/change/index` and bech32 addresses start with the HRP `atoi`, indicating the app is compiled for the testnet).

The Speculos simulator can be used with the test-program. [Here are instructions](https://github.com/iotaledger/ledger-iota-app/tree/develop/docker) how to set it up.

## Parameters

```
ledger iota tester 1.0
Thomas Pototschnig <microengineer18@gmail.com>

USAGE:
    cli [FLAGS] [OPTIONS]

FLAGS:
    -d, --dump               dump memory after tests
    -h, --help               Prints help information
    -s, --simulator          select the simulator as transport
    -n, --non-interactive    run the program in non-interactive mode for automatic testing
    -V, --version            Prints version information

OPTIONS:
    -f, --format <format>        user output format hex, bin, json (default) as output file format
    -l, --limit <limit>          maximum number of tests done
    -r, --recorder <recorder>    record APDU requests and responses to a file
```

The `dump` flag is a bit special because it does a complete memory dump after tests are run. It's only available if the app is compiled with `DEBUG=1` or `SPECULOS=1`. The main usage is to verify manually and visually that stack didn't grow into the data section. This is only of interest for people developing on the app.


The `bin`-Testfiles for CI-testing can be created by executing:

```
$ cli -n -s -r=reference.bin -f=bin
```

Please note that reference files for the Nano S and the Nano X differ because the `ledger.rs` lib uses a single-signing mode for the Nano S to save RAM, but uses another (faster) signing-mode on the Nano X. Also device information that can be read out via the `get_app_config` API-call are different.