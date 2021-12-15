//! Toggle the serial TX pin. Useful for verifying the delay implementation using a
//! logic analyzer.

#![no_main]
#![no_std]

use panic_persist as _;
use rtic::app;
use stm32l0xx_hal::{self as hal, pac, prelude::*};

use gfroerli_firmware::delay;

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
const APP: () = {
    #[init]
    fn init(ctx: init::Context) {
        let mut dp: pac::Peripherals = ctx.device;

        // Delay provider
        let mut delay = delay::Tim7Delay::new(dp.TIM7, &mut dp.RCC);

        // Clock configuration. Use HSI at 16 MHz.
        let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());

        // Toggle serial TX pin with delay
        let gpiob = dp.GPIOB.split(&mut rcc);
        let mut pin = gpiob.pb6.into_push_pull_output();

        // Trigger signal: Pull low for 100 µs, then high for 50 µs
        pin.set_low().unwrap();
        delay.delay_us(100);
        pin.set_high().unwrap();
        delay.delay_us(50);

        // Toggle with increasing durations
        for i in 1..=10 {
            pin.set_low().unwrap();
            delay.delay_us(i);
            pin.set_high().unwrap();
            delay.delay_us(i);
        }
        for i in 1..=10 {
            pin.set_low().unwrap();
            delay.delay_ms(i);
            pin.set_high().unwrap();
            delay.delay_ms(i);
        }
    }
};
