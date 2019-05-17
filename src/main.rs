//! Prints "Hello, world!" on the OpenOCD console using semihosting
#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::fmt::Write;

use cortex_m_rt::entry;
use cortex_m_semihosting as sh;
use sh::hio;
use stm32l0xx_hal as hal;
use stm32l0xx_hal::rcc::RccExt;

#[entry]
fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();

    let p = cortex_m::Peripherals::take().unwrap();
    let dp = stm32l0xx_hal::device::Peripherals::take().unwrap();

    writeln!(stdout, "Hello, world!").unwrap();

    writeln!(stdout, "Initializing Delay").unwrap();
    let syst = p.SYST;
    let rcc_config = hal::rcc::Config::default();
    let rcc = dp.RCC.freeze(rcc_config);
    let mut delay = hal::delay::Delay::new(syst, rcc.clocks);

    writeln!(stdout, "Starting loop").unwrap();
    loop {
        writeln!(stdout, "Hello, ").unwrap();
        delay.delay(hal::time::MicroSeconds(1_000_000));
        writeln!(stdout, "world!").unwrap();
        delay.delay(hal::time::MicroSeconds(1_000_000));
    }
}
