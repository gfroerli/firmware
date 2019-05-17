//! Prints "Hello, world!" on the OpenOCD console using semihosting
#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::fmt::Write;

use cortex_m::asm;
use cortex_m_rt::entry;
use cortex_m_semihosting as sh;
use embedded_hal;
use sh::hio;
use stm32l0xx_hal as hal;

#[entry]
fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();

    let _p = cortex_m::Peripherals::take().unwrap();

    writeln!(stdout, "Hello, world!").unwrap();

    loop {
        asm::wfi();
    }
}
