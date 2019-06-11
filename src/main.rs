//! Prints "Hello, world!" on the OpenOCD console using semihosting
#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::fmt::Write;

use cortex_m_rt::entry;
use cortex_m_semihosting as sh;
use sh::hio;
use stm32l0xx_hal as hal;
use stm32l0xx_hal::prelude::*;

#[entry]
fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();

    let p = cortex_m::Peripherals::take().unwrap();
    let dp = stm32l0xx_hal::device::Peripherals::take().unwrap();

    writeln!(stdout, "Hello, world!").unwrap();

    writeln!(stdout, "Initializing Delay").unwrap();
    let syst = p.SYST;
    let rcc_config = hal::rcc::Config::default();
    let mut rcc = dp.RCC.freeze(rcc_config);
    let mut delay = hal::delay::Delay::new(syst, rcc.clocks);

    writeln!(stdout, "Initializing GPIO").unwrap();
    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut led = gpiob.pb3.into_push_pull_output();

    writeln!(stdout, "Initializing USART").unwrap();
    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut serial = hal::serial::Serial::usart2(
        dp.USART2,
        (
            gpioa.pa9.into_floating_input(),
            gpioa.pa10.into_floating_input(),
        ),
        hal::serial::Config {
            baudrate: hal::time::Bps(9600),
            wordlength: hal::serial::WordLength::DataBits8,
            parity: hal::serial::Parity::ParityNone,
            stopbits: hal::serial::StopBits::STOP1,
        },
        &mut rcc,
    )
    .unwrap();

    writeln!(stdout, "Starting loop").unwrap();
    loop {
        writeln!(stdout, "Hello, ").unwrap();
        led.set_high();
        serial.write_char('a').unwrap();
        delay.delay(hal::time::MicroSeconds(1_000_000));
        writeln!(stdout, "world!").unwrap();
        led.set_low();
        serial.write_char('b').unwrap();
        delay.delay(hal::time::MicroSeconds(1_000_000));
    }
}
