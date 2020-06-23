//! Controlling the status LEDs.

use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{
    self as hal,
    gpio::{Output, PushPull},
};

/// Status LEDs.
pub struct StatusLeds {
    /// Red status LED
    led_r: hal::gpio::gpiob::PB<Output<PushPull>>,
    /// Yellow status LED
    led_y: hal::gpio::gpiob::PB<Output<PushPull>>,
    /// Green status LED
    led_g: hal::gpio::gpioa::PA<Output<PushPull>>,
}

impl StatusLeds {
    pub fn new(
        led_r: hal::gpio::gpiob::PB<Output<PushPull>>,
        led_y: hal::gpio::gpiob::PB<Output<PushPull>>,
        led_g: hal::gpio::gpioa::PA<Output<PushPull>>,
    ) -> Self {
        Self {
            led_r,
            led_y,
            led_g,
        }
    }

    pub fn enable_red(&mut self) {
        self.led_r.set_high().unwrap();
    }

    pub fn disable_red(&mut self) {
        self.led_r.set_low().unwrap();
    }

    pub fn enable_yellow(&mut self) {
        self.led_y.set_high().unwrap();
    }

    pub fn disable_yellow(&mut self) {
        self.led_y.set_low().unwrap();
    }

    pub fn enable_green(&mut self) {
        self.led_g.set_high().unwrap();
    }

    pub fn disable_green(&mut self) {
        self.led_g.set_low().unwrap();
    }

    pub fn disable_all(&mut self) {
        self.disable_red();
        self.disable_yellow();
        self.disable_green();
    }
}

/// Used by timer that cycles through LEDs.
pub(crate) enum LedState {
    Red,
    Yellow,
    Green,
}
