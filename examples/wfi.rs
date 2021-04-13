//! A firmware that does nothing but call WFI in a loop.
//! Used to test base current draw.
//!
//! To flash:
//!
//!     $ cargo embed flash --release --example wfi
//!
//! Make sure to power cycle the device afterwards, to prevent the debug
//! peripherals from running.

#![no_main]
#![no_std]
#![cfg(target_arch = "arm")]

use cortex_m_rt::entry;

use panic_persist as _;
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{self as hal, pac};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Clock configuration. Use HSI at 16 MHz.
    let _rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());

    // First, draw some power by loading the CPU (roughly 1.5s)
    for _ in 0..3_000_000 {
        cortex_m::asm::nop();
    }

    // Run WFI in a loop
    loop {
        cortex_m::asm::wfi();
    }
}
