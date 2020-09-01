//! Delay implementation using TIM7.
//!
//! This is done because RTIC takes ownership of SYST, and the stm32l0-hal by
//! default also wants SYST for its Delay implementation.
//!
//! TODO: Move this into the HAL?

use core::cmp::max;

use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use stm32l0xx_hal::pac;

pub struct Tim7Delay {
    tim7: pac::TIM7,
}

impl Tim7Delay {
    pub fn new(tim7: pac::TIM7, rcc: &mut pac::RCC) -> Self {
        // Enable TIM7 in RCC
        rcc.apb1enr.modify(|_, w| w.tim7en().set_bit());

        // Reset timer
        rcc.apb1rstr.modify(|_, w| w.tim7rst().set_bit());
        rcc.apb1rstr.modify(|_, w| w.tim7rst().clear_bit());

        // Enable one-pulse mode (counter stops counting at the next update
        // event, clearing the CEN bit)
        tim7.cr1.modify(|_, w| w.opm().enabled());

        Self { tim7 }
    }

    pub fn free(self) -> pac::TIM7 {
        self.tim7
    }
}

fn wait(tim7: &mut pac::TIM7, prescaler: u16, auto_reload_register: u16) {
    // Set up prescaler
    //
    // This implementation assumes that the core clock is set to 16 MHz (see
    // `CORE_CLOCK` constant). At 16 MHz, this means 62.5 ns per clock cycle.
    // By setting an appropriate prescaler, we can reduce the timer tick to the
    // desired duration (e.g. 16 for 1 µs or 16_000 for 1 ms).
    tim7.psc.write(|w| w.psc().bits(prescaler));

    // Set the auto-reload register to the delay value
    tim7.arr
        .write(|w| unsafe { w.arr().bits(max(1, auto_reload_register)) });

    // Trigger update event (UEV) in the event generation register (EGR)
    // in order to immediately apply the config
    tim7.egr.write(|w| w.ug().set_bit());

    // Enable counter
    tim7.cr1.modify(|_, w| w.cen().set_bit());

    // Wait for CEN bit to clear
    while tim7.cr1.read().cen().is_enabled() { /* wait */ }
}

/// Note that values below 4 µs are too short to be reliably measured using
/// this implementation. The delay will always be at least ~4 µs long.
impl DelayUs<u16> for Tim7Delay {
    fn delay_us(&mut self, us: u16) {
        // We have roughly 3 µs overhead. Note that this was only measured
        // experimentally using a cheap logic analyzer and might be slightly off.
        let overhead = 3;

        // Subtract 1 from the ARR register because the update event is
        // triggered *after* that tick.
        wait(
            &mut self.tim7,
            16,
            if us > overhead { us - overhead - 1 } else { 0 },
        );
    }
}

impl DelayMs<u16> for Tim7Delay {
    fn delay_ms(&mut self, ms: u16) {
        // If we set the ARR register to 0 (1-1), then the update event
        // will never trigger. Therefore delegate to the `DelayUs` impl.
        if ms <= 1 {
            self.delay_us(ms * 1000);
            return;
        }

        // Subtract 1 from the ARR register because the update event is
        // triggered *after* that tick.
        wait(&mut self.tim7, 16_000, ms - 1);
    }
}
