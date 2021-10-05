use embedded_hal::adc::OneShot;
use embedded_hal::digital::v2::OutputPin;
use stm32l0xx_hal::adc::{self, Adc, Align, VRef};
use stm32l0xx_hal::gpio::gpioa::{PA, PA1};
use stm32l0xx_hal::gpio::{Analog, Output, PushPull};

use gfroerli_common::measurement::U12;

/// Manages the supply voltage monitoring circuit
pub struct SupplyMonitor {
    adc_pin: PA1<Analog>,
    adc: Adc<adc::Ready>,
    enable_pin: PA<Output<PushPull>>,
}

impl SupplyMonitor {
    const ADC_MAX: f32 = 4095.0;
    const VREFINT_VOLTAGE: f32 = 1.224;

    pub fn new(
        adc_pin: PA1<Analog>,
        mut adc: Adc<adc::Ready>,
        enable_pin: PA<Output<PushPull>>,
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

    /// Read the VREFINT value
    pub fn read_vref_raw(&mut self) -> Option<u16> {
        VRef.enable(&mut self.adc);
        let val = self.adc.read(&mut VRef).ok();
        VRef.disable(&mut self.adc);
        val
    }

    /// Convert the raw VREFINT ADC value to VDDA
    pub fn convert_vrefint_to_vdda(vrefint: u16) -> f32 {
        Self::ADC_MAX * Self::VREFINT_VOLTAGE / (vrefint as f32)
    }

    /// Read VREFINT and calculate VDDA from it
    pub fn read_vdda(&mut self) -> Option<f32> {
        self.read_vref_raw().map(Self::convert_vrefint_to_vdda)
    }

    /// Read the supply voltage (see `read_supply_raw` for details) and return
    /// the raw data as `U12`.
    pub fn read_supply_raw_u12(&mut self) -> Option<U12> {
        self.read_supply_raw().map(U12::new)
    }

    /// Read the supply voltage (see `read_supply_raw` for details) and return
    /// the voltage in volts as `f32`.
    pub fn read_supply(&mut self) -> Option<f32> {
        let val = self.read_supply_raw()?;
        let vdda = self.read_vdda()?;
        Some(Self::convert_input(val, vdda))
    }

    /// Convert the raw ADC value to the resulting supply voltage
    pub fn convert_input(input: u16, vdda: f32) -> f32 {
        const R_1: f32 = 2.7;
        const R_2: f32 = 10.0;
        (input as f32) / Self::ADC_MAX * vdda / R_1 * (R_1 + R_2)
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
