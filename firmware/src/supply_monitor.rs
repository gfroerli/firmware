use embedded_hal::{adc::OneShot, digital::v2::OutputPin};
use stm32l0xx_hal::{
    adc::{self, Adc, Align},
    gpio::{gpioa::PA1, Analog, Output, Pin, PushPull},
};

use gfroerli_common::measurement::U12;

/// Manages the supply voltage monitoring circuit
pub struct SupplyMonitor {
    adc_pin: PA1<Analog>,
    adc: Adc<adc::Ready>,
    enable_pin: Pin<Output<PushPull>>,
}

impl SupplyMonitor {
    pub fn new(
        adc_pin: PA1<Analog>,
        mut adc: Adc<adc::Ready>,
        enable_pin: Pin<Output<PushPull>>,
    ) -> Self {
        adc.set_precision(adc::Precision::B_12);
        adc.set_align(Align::Right); // Use 12 least-significant bits to encode data
        adc.set_sample_time(adc::SampleTime::T_79_5);
        SupplyMonitor {
            adc_pin,
            adc,
            enable_pin,
        }
    }

    /// Disable the supply voltage monitoring voltage divider
    fn disable(&mut self) {
        self.enable_pin.set_low().unwrap();
    }

    /// Enable the supply voltage monitoring voltage divider
    fn enable(&mut self) {
        self.enable_pin.set_high().unwrap();
    }

    /// Read the supply voltage ADC channel.
    ///
    /// `enable` and `disable` the supply voltage monitoring voltage divider
    /// before and after the measurement.
    pub fn read_supply_raw(&mut self) -> Option<u16> {
        self.enable();
        let val: Option<u16> = self.adc.read(&mut self.adc_pin).ok();
        self.disable();
        val
    }

    /// Read the supply voltage (see `read_supply_raw` for details) and return
    /// the raw data as `U12`.
    pub fn read_supply_raw_u12(&mut self) -> Option<U12> {
        self.read_supply_raw().map(U12::new)
    }

    /// Read the supply voltage (see `read_supply_raw` for details) and return
    /// the voltage in volts as `f32`.
    pub fn read_supply_f32(&mut self) -> Option<f32> {
        let val = self.read_supply_raw()?;
        Some(Self::convert_input(val))
    }

    /// Read the supply voltage (see `read_supply_raw` for details) and return
    /// the voltage in millivolts with 2 V offset as `U12`.
    ///
    /// To convert this back to volts, use the formula `val / 1000 + 2`.
    pub fn read_supply_u12(&mut self) -> Option<U12> {
        let val = self.read_supply_raw()?;
        let volts = Self::convert_input(val);
        let millivolts_with_offset = ((volts - 2.0) * 1000.0) as u16;
        Some(U12::new(millivolts_with_offset))
    }

    /// Convert the raw ADC value to the resulting supply voltage
    pub fn convert_input(input: u16) -> f32 {
        const SUPPLY_VOLTAGE: f32 = 3.3;
        const ADC_MAX: f32 = 4095.0;
        const R_1: f32 = 9.31;
        const R_2: f32 = 6.04;
        (input as f32) / ADC_MAX * SUPPLY_VOLTAGE / R_1 * (R_1 + R_2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_input() {
        let inputs = [0, 2047, 4095];
        let results = [0.0, 2.72, 5.44];
        for (input, expected) in inputs.iter().zip(&results) {
            let result = SupplyMonitor::convert_input(*input);
            println!("{} -> {}, should {}", input, result, expected);
            assert!((result - *expected).abs() < 0.01);
        }
    }
}
