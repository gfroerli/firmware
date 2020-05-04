#![no_main]
#![no_std]

extern crate panic_halt;

use core::fmt::Write;

use rtfm::app;
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{
    self as hal,
    gpio::{Output, PushPull},
    pac, serial, time,
};

#[app(device = stm32l0::stm32l0x1, peripherals = true)]
const APP: () = {
    struct Resources {
        /// Serial debug output
        debug: hal::serial::Serial<pac::USART1>,

        /// Red status LED
        led_r: hal::gpio::gpiob::PB<Output<PushPull>>,
        /// Yellow status LED
        led_y: hal::gpio::gpiob::PB<Output<PushPull>>,
        /// Green status LED
        led_g: hal::gpio::gpioa::PA<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let _p: cortex_m::Peripherals = ctx.core;
        let dp: pac::Peripherals = ctx.device;

        let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());

        let gpioa = dp.GPIOA.split(&mut rcc);
        let gpiob = dp.GPIOB.split(&mut rcc);

        // Initialize serial port(s)
        let mut debug = serial::Serial::usart1(
            dp.USART1,
            gpiob.pb6.into_floating_input(),
            gpiob.pb7.into_floating_input(),
            serial::Config {
                baudrate: time::Bps(57_600),
                wordlength: serial::WordLength::DataBits8,
                parity: serial::Parity::ParityNone,
                stopbits: serial::StopBits::STOP1,
            },
            &mut rcc,
        )
        .unwrap();

        writeln!(debug, "Greetings, Rusty world, from Gfrörli v2!").unwrap();
        writeln!(debug, "Debug output initialized on USART1!").unwrap();

        // Initialize LEDs
        let mut led_r = gpiob.pb1.into_push_pull_output().downgrade();
        let mut led_y = gpiob.pb0.into_push_pull_output().downgrade();
        let mut led_g = gpioa.pa7.into_push_pull_output().downgrade();
        led_r.set_high().unwrap();
        led_y.set_low().unwrap();
        led_g.set_high().unwrap();

        //        writeln!(debug, "Starting loop").unwrap();
        //        loop {
        //            write!(debug, "a").unwrap();
        //
        //            led_r.set_high().expect("Could not turn on LED");
        //            delay.delay(time::MicroSeconds(100_000));
        //            led_y.set_high().expect("Could not turn on LED");
        //            delay.delay(time::MicroSeconds(100_000));
        //            led_g.set_high().expect("Could not turn on LED");
        //
        //            delay.delay(time::MicroSeconds(200_000));
        //
        //            write!(debug, "b").unwrap();
        //
        //            led_r.set_low().expect("Could not turn off LED");
        //            delay.delay(time::MicroSeconds(100_000));
        //            led_y.set_low().expect("Could not turn off LED");
        //            delay.delay(time::MicroSeconds(100_000));
        //            led_g.set_low().expect("Could not turn off LED");
        //
        //            delay.delay(time::MicroSeconds(200_000));
        //        }

        init::LateResources {
            debug,
            led_r,
            led_y,
            led_g,
        }
    }

    // RTFM requires that free interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    extern "C" {
        fn SPI1();
        fn SPI2();
    }
};
