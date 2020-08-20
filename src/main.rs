#![no_main]
#![no_std]

extern crate panic_halt;

use core::fmt::Write;

use ds18b20::Ds18b20;
use one_wire_bus::OneWire;
use rtic::app;
use shtcx::{shtc3, LowPower, PowerMode, ShtC3};
use stm32l0::stm32l0x1::I2C1;
use stm32l0xx_hal::gpio::{
    gpioa::{PA10, PA6, PA9},
    OpenDrain, Output,
};
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{self as hal, i2c::I2c, pac, serial, time};

mod delay;
mod ds18b20_utils;
mod leds;
mod version;

use delay::Tim7Delay;
use leds::{LedState, StatusLeds};
use version::HardwareVersionDetector;

const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

enum ShtState {
    StartMeasurement,
    ReadResults,
}

#[app(device = stm32l0::stm32l0x1, peripherals = true)]
const APP: () = {
    struct Resources {
        /// Serial debug output
        debug: hal::serial::Serial<pac::USART1>,

        status_leds: StatusLeds,
        timer: hal::timer::Timer<pac::TIM6>,
        measurement_timer: hal::timer::Timer<pac::TIM3>,
        sht: ShtC3<I2c<I2C1, PA10<Output<OpenDrain>>, PA9<Output<OpenDrain>>>>,
        one_wire: OneWire<PA6<Output<OpenDrain>>>,
        ds18b20: Ds18b20,
        delay: Tim7Delay,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let _p: cortex_m::Peripherals = ctx.core;
        let dp: pac::Peripherals = ctx.device;

        // Clock configuration. Use HSI at 16 MHz.
        let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());

        // TODO: Use the MSI (Multispeed Internal) clock source instead.
        // However, currently the timer cannot be initialized (at least when
        // connected through the debugger) when enabling MSI.
        //let mut rcc = dp.RCC.freeze(
        //    hal::rcc::Config::msi(hal::rcc::MSIRange::Range5) // ~2.097 MHz
        //);

        // Init delay timer
        let mut delay = delay::Tim7Delay::new(dp.TIM7);

        // Initialize timer to blink LEDs. Use TIM6 since it has lower current
        // consumption than TIM2/3 or TIM21/22.
        let mut timer = hal::timer::Timer::tim6(dp.TIM6, 2.hz(), &mut rcc);
        timer.listen();

        let mut measurement_timer = hal::timer::Timer::tim3(dp.TIM3, 2.hz(), &mut rcc);
        measurement_timer.listen();

        // Get access to GPIOs
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

        // Initialize version detector
        let mut hardware_version = HardwareVersionDetector::new(
            gpioa.pa12.into_floating_input(),
            gpiob.pb4.into_floating_input(),
            gpiob.pb5.into_floating_input(),
        );

        // Show versions
        writeln!(
            debug,
            "Gfrörli firmware={} hardware={}",
            FIRMWARE_VERSION,
            hardware_version.detect(),
        )
        .unwrap();

        let one_wire_pin = gpioa.pa6.into_open_drain_output();
        let mut one_wire =
            ds18b20_utils::create_and_scan_one_wire(&mut delay, &mut debug, one_wire_pin);
        let ds18b20 = ds18b20_utils::get_ds18b20_sensor(&mut delay, &mut one_wire)
            .expect("Could not find ds18b20 sensor");

        // Initialize LEDs
        let mut status_leds = StatusLeds::new(
            gpiob.pb1.into_push_pull_output().downgrade(),
            gpiob.pb0.into_push_pull_output().downgrade(),
            gpioa.pa7.into_push_pull_output().downgrade(),
        );
        status_leds.disable_all();

        let sda = gpioa.pa10.into_open_drain_output();
        let scl = gpioa.pa9.into_open_drain_output();
        let i2c = dp.I2C1.i2c(sda, scl, 10.khz(), &mut rcc);

        let mut sht = shtc3(i2c);
        sht.wakeup(&mut delay)
            .expect("SHTCx: Could not wake up sensor");

        writeln!(debug, "Initialization done").unwrap();

        init::LateResources {
            debug,
            status_leds,
            timer,
            sht,
            measurement_timer,
            one_wire,
            ds18b20,
            delay,
        }
    }

    #[task(binds = TIM6, resources = [status_leds, timer])]
    fn timer(ctx: timer::Context) {
        static mut STATE: LedState = LedState::Green;

        // Clear the interrupt flag
        ctx.resources.timer.clear_irq();

        // Change LED on every interrupt
        match STATE {
            LedState::Green => {
                ctx.resources.status_leds.disable_green();
                ctx.resources.status_leds.enable_red();
                *STATE = LedState::Red;
            }
            LedState::Red => {
                ctx.resources.status_leds.disable_red();
                ctx.resources.status_leds.enable_yellow();
                *STATE = LedState::Yellow;
            }
            LedState::Yellow => {
                ctx.resources.status_leds.disable_yellow();
                ctx.resources.status_leds.enable_green();
                *STATE = LedState::Green;
            }
        }
    }

    #[task(binds = TIM3, resources = [measurement_timer, sht, debug, one_wire, delay, ds18b20])]
    fn measurement_timer(mut ctx: measurement_timer::Context) {
        static mut STATE: ShtState = ShtState::StartMeasurement;

        // Clear the interrupt flag
        ctx.resources.measurement_timer.clear_irq();

        *STATE = match STATE {
            ShtState::StartMeasurement => {
                ctx.resources
                    .sht
                    .start_measurement(PowerMode::NormalMode)
                    .expect("SHTCx: Failed to start measurement");
                ctx.resources
                    .ds18b20
                    .start_temp_measurement(&mut ctx.resources.one_wire, ctx.resources.delay)
                    .expect("DS18B20: Failed to start measurement");
                ShtState::ReadResults
            }
            ShtState::ReadResults => {
                let measurement = ctx
                    .resources
                    .sht
                    .get_measurement_result()
                    .expect("SHTCx: Failed to read measurement");
                let ds18b20_measurement = ctx
                    .resources
                    .ds18b20
                    .read_data(&mut ctx.resources.one_wire, ctx.resources.delay)
                    .expect("DS18B20: Failed to read measurement");
                writeln!(
                    ctx.resources.debug,
                    "{:.2} °C ({:?}), {:.2} °C, {:.2} %RH",
                    ds18b20_measurement.temperature,
                    ds18b20_measurement.resolution,
                    measurement.temperature.as_degrees_celsius(),
                    measurement.humidity.as_percent()
                )
                .unwrap();
                ShtState::StartMeasurement
            }
        }
    }

    // RTIC requires that free interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    extern "C" {
        fn SPI1();
        fn SPI2();
    }
};
