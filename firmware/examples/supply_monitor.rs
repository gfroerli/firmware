//! Prints the supply voltage monitor output to the serial port.
#![no_main]
#![no_std]

use panic_persist as _;

use core::fmt::Write;

use cortex_m_rt::entry;
use embedded_time::rate::Baud;
use hal::serial;
use stm32l0xx_hal as hal;
use stm32l0xx_hal::prelude::*;

use gfroerli_firmware::supply_monitor;

#[entry]
fn main() -> ! {
    let p = cortex_m::Peripherals::take().unwrap();
    let dp = stm32l0xx_hal::pac::Peripherals::take().unwrap();

    let syst = p.SYST;
    let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());
    let mut delay = hal::delay::Delay::new(syst, rcc.clocks);

    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut led = gpiob.pb3.into_push_pull_output();

    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut serial = hal::serial::Serial::usart1(
        dp.USART1,
        gpiob.pb6.into_floating_input(),
        gpiob.pb7.into_floating_input(),
        serial::Config {
            baudrate: Baud(57_600),
            wordlength: serial::WordLength::DataBits8,
            parity: serial::Parity::ParityNone,
            stopbits: serial::StopBits::STOP1,
        },
        &mut rcc,
    )
    .unwrap();

    writeln!(serial, "Starting supply_monitor example").unwrap();

    // Initialize supply monitor
    let adc = dp.ADC.constrain(&mut rcc);

    let a1 = gpioa.pa1.into_analog();
    let adc_enable_pin = gpioa.pa5.into_push_pull_output().downgrade();
    let mut supply_monitor = supply_monitor::SupplyMonitor::new(a1, adc, adc_enable_pin);

    loop {
        let v_supply = supply_monitor.read_supply_raw();
        if let Some(v_supply_raw) = v_supply {
            let v_input = (v_supply_raw as f32) / 4095.0 * 3.3;
            let v_supply_converted = supply_monitor::SupplyMonitor::convert_input(v_supply_raw);

            writeln!(
                serial,
                "Raw: {} ({}) -> Supply: {}",
                v_supply_raw, v_input, v_supply_converted
            )
            .unwrap();
        }

        led.set_high().unwrap();
        delay.delay_ms(1_000u16);
        led.set_low().unwrap();
        delay.delay_ms(1_000u16);
    }
}
