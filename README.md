# water-sensor-firmware-rs

## Build

```Bash
# Switch to the nightly channel
rustup override set nightly

# Install dependencies
sudo pacman -S arm-none-eabi-binutils arm-none-eabi-gdb openocd

# Install xargo
cargo install xargo
rustup component add rust-src

xargo build
```

## Run

```Bash
./openocd.sh
arm-none-eabi-gdb target/thumbv6m-none-eabi/debug/water-sensor-firmware
```

## License

Licensed under GPL-3.0 license ([LICENSE.md](LICENSE.md) or
https://www.gnu.org/licenses/gpl-3.0.en.html)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any
additional terms or conditions.
