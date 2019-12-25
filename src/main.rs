//! Prints "Hello, world!" on the OpenOCD console using semihosting
#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::fmt::Write;

use cortex_m_rt::entry;
use cortex_m_semihosting as sh;
use rn2xx3::rn2483_868;
use sh::hio;
use stm32l0xx_hal as hal;
use stm32l0xx_hal::prelude::*;

#[entry]
#[allow(clippy::missing_safety_doc)]
fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();

    let p = cortex_m::Peripherals::take().unwrap();
    let dp = hal::pac::Peripherals::take().unwrap();

    writeln!(stdout, "Greetings, Rusty world, from Gfrörli v2!").unwrap();

    writeln!(stdout, "Initializing Delay").unwrap();
    let syst = p.SYST;
    let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());
    let mut delay = hal::delay::Delay::new(syst, rcc.clocks);

    writeln!(stdout, "Initializing GPIO").unwrap();
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut led_r = gpiob.pb1.into_push_pull_output();
    let mut led_y = gpiob.pb0.into_push_pull_output();
    let mut led_g = gpioa.pa7.into_push_pull_output();

    writeln!(stdout, "Initializing RN2483 driver").unwrap();
    // Config: See RN2483 datasheet, table 3-1
    let rn_serial_config = hal::serial::Config {
        baudrate: hal::time::Bps(57_600),
        wordlength: hal::serial::WordLength::DataBits8,
        parity: hal::serial::Parity::ParityNone,
        stopbits: hal::serial::StopBits::STOP1,
    };
    let rn_serial = hal::serial::Serial::lpuart1(
        dp.LPUART1,
        (gpioa.pa2, gpioa.pa3),
        rn_serial_config,
        &mut rcc,
    )
    .expect("Invalid LPUART1 config");
    let mut rn = rn2483_868(rn_serial);

    writeln!(stdout, "RN2483: Resetting module").unwrap();
    let version = rn.reset().expect("Could not reset");
    writeln!(stdout, "RN2483: Version {}", version).unwrap();

    //writeln!(stdout, "Initializing USART").unwrap();
    //let gpioa = dp.GPIOA.split(&mut rcc);
    //let mut serial = hal::serial::Serial::usart2(
    //    dp.USART2,
    //    (
    //        gpioa.pa9.into_floating_input(),
    //        gpioa.pa10.into_floating_input(),
    //    ),
    //    hal::serial::Config {
    //        baudrate: hal::time::Bps(9600),
    //        wordlength: hal::serial::WordLength::DataBits8,
    //        parity: hal::serial::Parity::ParityNone,
    //        stopbits: hal::serial::StopBits::STOP1,
    //    },
    //    &mut rcc,
    //)
    //.unwrap();

    writeln!(stdout, "Starting loop").unwrap();
    loop {
        writeln!(stdout, "Hello, ").unwrap();

        //serial.write_char('a').unwrap();
        //serial.write_char('b').unwrap();

        led_r.set_high().expect("Could not turn on LED");
        delay.delay(hal::time::MicroSeconds(100_000));
        led_y.set_high().expect("Could not turn on LED");
        delay.delay(hal::time::MicroSeconds(100_000));
        led_g.set_high().expect("Could not turn on LED");

        delay.delay(hal::time::MicroSeconds(200_000));

        writeln!(stdout, "world!").unwrap();

        led_r.set_low().expect("Could not turn off LED");
        delay.delay(hal::time::MicroSeconds(100_000));
        led_y.set_low().expect("Could not turn off LED");
        delay.delay(hal::time::MicroSeconds(100_000));
        led_g.set_low().expect("Could not turn off LED");

        delay.delay(hal::time::MicroSeconds(200_000));
    }
}
