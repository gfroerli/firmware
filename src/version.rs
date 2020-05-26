//! Detecting the hardware version.

use core::fmt;

use stm32l0xx_hal::gpio::gpioa::PA12;
use stm32l0xx_hal::gpio::gpiob::{PB4, PB5};
use stm32l0xx_hal::gpio::{Floating, Input};
use stm32l0xx_hal::prelude::*;

pub enum HardwareVersion {
    V2,
    Unknown(u8),
}

impl fmt::Display for HardwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V2 => write!(f, "2.0"),
            Self::Unknown(v) => write!(f, "v?({})", v),
        }
    }
}

pub struct HardwareVersionDetector {
    bit0: PA12<Input<Floating>>,
    bit1: PB4<Input<Floating>>,
    bit2: PB5<Input<Floating>>,
}

impl HardwareVersionDetector {
    pub fn new(
        pa12: PA12<Input<Floating>>,
        pb4: PB4<Input<Floating>>,
        pb5: PB5<Input<Floating>>,
    ) -> Self {
        Self {
            bit0: pa12,
            bit1: pb4,
            bit2: pb5,
        }
    }

    /// Detect the hardware version (number between 0 and 15).
    pub fn detect_raw(&mut self) -> u8 {
        let bit0 = self.bit0.is_high().unwrap() as u8;
        let bit1 = self.bit1.is_high().unwrap() as u8;
        let bit2 = self.bit2.is_high().unwrap() as u8;
        bit0 | (bit1 << 1) | (bit2 << 2)
    }

    /// Detect the hardware version.
    pub fn detect(&mut self) -> HardwareVersion {
        match self.detect_raw() {
            0 => HardwareVersion::V2,
            other => HardwareVersion::Unknown(other),
        }
    }
}
