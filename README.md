# Gfrör.li v2 Firmware

## Subcrates

- `firmware`: The main firmware repository. This will generate the firmware binary.
- `common`: Common code like configuration management in EEPROM.
- `cli-utils`: Utilities like `config-flasher` A tool that can write config
  EEPROM based on a config TOML file.


## Timers

We use the following timers:

- `LPTIM`: Used as a `Monotonic` implementation for the RTIC scheduling system
- `TIM7`: Used as a blocking delay provider for the device drivers


## Prerequisites

Requires Rust stable, as specified in the `rust-toolchain` file.

```Bash
# Install dependencies
sudo pacman -S arm-none-eabi-binutils arm-none-eabi-gdb
# For debugging we need JLinkGDBServer
yay jlink

# Install target
rustup target add thumbv6m-none-eabi
```

## Subcrate: Firmware

### Build

To build the firmware:

    cargo build --release

### Run & debug

In one terminal:

    JLinkGDBServer -speed 4000 -if SWD -device STM32L071KB -port 3333

In another terminal:

    cargo run

### Flash

Use `cargo-embed` (0.8+):

    cargo embed flash --release

To reset the target without flashing (requires `cargo-embed` 0.9+):

    cargo embed reset

To enable more verbose debug logging (e.g. with conversion of raw temperature
data) as well as some debug assertions, enable the `dev` features

    cargo embed flash --release --features dev

### Run Unit Tests

Unit tests run on the host system, so we need to specify the target accordingly:
```
cargo test --target x86_64-unknown-linux-gnu --tests
cargo test --target x86_64-unknown-linux-gnu --tests --features dev
```

## Tools

### Config Flasher

First, prepare a config file like this:

```toml
# config.toml
version = 1
devaddr = "00000000"
nwkskey = "11111111111111111111111111111111"
appskey = "22222222222222222222222222222222"
wakeup_interval_seconds = 900
nth_temp_humi = 1
nth_voltage = 4
```

Then flash it to the attached board:

    cargo run --bin config-flasher -- --config config.toml


## [TTN](./docs/ttn.md)

## License

Licensed under GPL-3.0 license ([LICENSE.md](LICENSE.md) or
https://www.gnu.org/licenses/gpl-3.0.en.html)

**Contributions**

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any
additional terms or conditions.
