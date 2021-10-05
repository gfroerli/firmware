//! Prints the supply voltage monitor output to the serial port.
#![no_main]
#![no_std]

use panic_persist as _;

use core::fmt::Write;

use cortex_m_rt::entry;
use stm32l0xx_hal as hal;
use stm32l0xx_hal::prelude::*;

use gfroerli_firmware::supply_monitor;
use gfroerli_common::measurement::U12;

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
    let mut serial = hal::serial::Serial::usart2(
        dp.USART2,
        gpioa.pa2.into_floating_input(),
        gpioa.pa3.into_floating_input(),
        
        //gpioa.pa9.into_floating_input(),
        //gpioa.pa10.into_floating_input(),
        
        //gpiob.pb6.into_floating_input(),
        //gpiob.pb7.into_floating_input(),
        hal::serial::Config {
            baudrate: hal::time::Bps(9600),
            wordlength: hal::serial::WordLength::DataBits8,
            parity: hal::serial::Parity::ParityNone,
            stopbits: hal::serial::StopBits::STOP1,
        },
        &mut rcc,
    )
    .unwrap();

    // Initialize supply monitor
    let adc = dp.ADC.constrain(&mut rcc);
    let a1 = gpioa.pa1.into_analog();
    let adc_enable_pin = gpioa.pa5.into_push_pull_output().downgrade();
    let mut supply_monitor = supply_monitor::SupplyMonitor::new(a1, adc, adc_enable_pin);


    loop {
        let v_supply = supply_monitor.read_supply_raw();
        if let Some(v) = v_supply {
            writeln!(serial, "{:?}", v);

            // real ca. 0.7V@3.3V
            let v_input =  (v as f32) / 4095.0 * 3.3;
            let v_supply_converted =  (v as f32) / 4095.0 * 3.3 / 2.7 * (10.0 + 2.7);

            writeln!(serial, "{} -> {:?}", v_input, v_supply_converted);
        }

        //serial.write_char('a').unwrap();

        led.set_high().unwrap();
        delay.delay(hal::time::MicroSeconds(1_000_000));
        led.set_low().unwrap();
        delay.delay(hal::time::MicroSeconds(1_000_000));
    }
}
