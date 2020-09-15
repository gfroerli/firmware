use embedded_hal::adc::OneShot;
use embedded_hal::digital::v2::OutputPin;
use stm32l0xx_hal::adc::{self, Adc};
use stm32l0xx_hal::gpio::gpioa::{PA, PA1};
use stm32l0xx_hal::gpio::{Analog, Output, PushPull};

pub struct SupplyMonitor {
    adc_pin: PA1<Analog>,
    adc: Adc<adc::Ready>,
    enable_pin: PA<Output<PushPull>>,
}

impl SupplyMonitor {
    pub fn new(
        adc_pin: PA1<Analog>,
        mut adc: Adc<adc::Ready>,
        enable_pin: PA<Output<PushPull>>,
    ) -> Self {
        adc.set_precision(adc::Precision::B_12);
        adc.set_sample_time(adc::SampleTime::T_79_5);
        SupplyMonitor {
            adc_pin,
            adc,
            enable_pin,
        }
    }

    fn disable(&mut self) {
        self.enable_pin.set_low().unwrap();
    }

    fn enable(&mut self) {
        self.enable_pin.set_high().unwrap();
    }

    pub fn read_supply_raw(&mut self) -> Option<u16> {
        self.enable();
        let val: Option<u16> = self.adc.read(&mut self.adc_pin).ok();
        self.disable();
        val
    }

    pub fn read_supply(&mut self) -> Option<f32> {
        let val = self.read_supply_raw()?;
        Some(Self::convert_input(val))
    }

    pub fn convert_input(input: u16) -> f32 {
        const SUPPLY_VOLTAGE: f32 = 3.3;
        const ADC_MAX: f32 = 4095.0;
        const R_1: f32 = 9.31;
        const R_2: f32 = 6.04;
        (input as f32) / ADC_MAX * SUPPLY_VOLTAGE / R_1 * (R_1 + R_2)
    }
}
