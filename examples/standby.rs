//! A firmware that does nothing but putting the device into standby mode.
//! Used to test standby current draw.
//!
//! To flash:
//!
//!     $ cargo embed flash --release --example standby
//!
//! Make sure to power cycle the device afterwards, to prevent the debug
//! peripherals from running.

#![no_main]
#![no_std]
#![cfg(target_arch = "arm")]

use core::time::Duration;

use cortex_m_rt::entry;

use panic_persist as _;
use rn2xx3::rn2483_868;
use shtcx::{shtc3, LowPower};
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{self as hal, pac, serial, time};

#[entry]
fn main() -> ! {
    // Peripherals
    let mut cp = pac::CorePeripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    // Clock configuration. Use HSI at 16 MHz.
    let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());

    // Get access to PWR peripheral
    let mut pwr = hal::pwr::PWR::new(dp.PWR, &mut rcc);

    // Get access to GPIOs
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Set up I²C pins
    let sda = gpioa.pa10.into_open_drain_output();
    let scl = gpioa.pa9.into_open_drain_output();
    let i2c = dp.I2C1.i2c(sda, scl, 10.khz(), &mut rcc);

    // Initialize serial port
    let mut lpuart1 = serial::Serial::lpuart1(
        dp.LPUART1,
        gpioa.pa2.into_floating_input(),
        gpioa.pa3.into_floating_input(),
        // Config: See RN2483 datasheet, table 3-1
        serial::Config {
            baudrate: time::Bps(57_600),
            wordlength: serial::WordLength::DataBits8,
            parity: serial::Parity::ParityNone,
            stopbits: serial::StopBits::STOP1,
        },
        &mut rcc,
    )
    .unwrap();

    // Clear LPUART1 hardware error flags
    lpuart1.clear_errors();

    // Initialize RN2xx3
    let mut rn = rn2483_868(lpuart1);

    // Busy loop, to give us a chance to re-program without reset loop
    for _ in 0..3_000_000 {
        cortex_m::asm::nop();
    }

    // Put SHTC3 into sleep mode
    let mut sht = shtc3(i2c);
    sht.sleep().unwrap();

    // Short busy loop
    for _ in 0..200_000 {
        cortex_m::asm::nop();
    }

    // Drain RN2483 serial buffer and ensure it's working
    rn.ensure_known_state().unwrap();

    // Put RN2483 into sleep mode
    rn.sleep(Duration::from_secs(5)).unwrap();

    // Long busy loop
    for _ in 0..6_000_000 {
        cortex_m::asm::nop();
    }

    // Put device into standby
    let mut standby = pwr.standby_mode(&mut cp.SCB);
    loop {
        standby.enter();
    }
}
