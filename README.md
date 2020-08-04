# Gfrör.li v2 Firmware


## Build

Requires Rust stable, as specified in the `rust-toolchain` file.

```Bash
# Install dependencies
sudo pacman -S arm-none-eabi-binutils arm-none-eabi-gdb openocd

# Install target
rustup target add thumbv6m-none-eabi

cargo build --release
```


## Run & debug

In one terminal:

    ./openocd.sh

In another terminal:

    cargo run


## Flash

Use `cargo-embed` (0.8+):

    cargo embed flash --release

To reset the target without flashing (requires `cargo-embed` 0.9+):

    cargo embed reset


## License

Licensed under GPL-3.0 license ([LICENSE.md](LICENSE.md) or
https://www.gnu.org/licenses/gpl-3.0.en.html)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any
additional terms or conditions.
