#![no_main]
#![no_std]
#![cfg(target_arch = "arm")]

use core::fmt::Write;

use one_wire_bus::OneWire;
use panic_persist as _;
use rtic::app;
use shtcx::{shtc3, LowPower, PowerMode, ShtC3};
use stm32l0xx_hal::gpio::{
    gpioa::{PA10, PA6, PA9},
    OpenDrain, Output,
};
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{self as hal, i2c::I2c, pac, serial, time};

type I2C1 = I2c<stm32l0::stm32l0x1::I2C1, PA10<Output<OpenDrain>>, PA9<Output<OpenDrain>>>;

mod config;
mod delay;
mod ds18b20;
mod leds;
mod measurement;
mod monotonic_stm32l0;
mod supply_monitor;
mod version;

use config::Config;
use delay::Tim7Delay;
use ds18b20::Ds18b20;
use leds::{LedState, StatusLeds};
use monotonic_stm32l0::{Tim6Monotonic, U16Ext};
use supply_monitor::SupplyMonitor;
use version::HardwareVersionDetector;

const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[app(
    device = stm32l0::stm32l0x1,
    peripherals = true,
    monotonic = crate::monotonic_stm32l0::Tim6Monotonic,
)]
const APP: () = {
    struct Resources {
        // Serial debug output
        debug: hal::serial::Serial<pac::USART1>,

        // Status LEDs
        status_leds: StatusLeds,

        // SHT temperature/humidity sensor
        sht: ShtC3<I2C1>,

        // DS18B20 water temperature sensor
        one_wire: OneWire<PA6<Output<OpenDrain>>>,
        ds18b20: Ds18b20,

        // Blocking delay provider
        delay: Tim7Delay,
    }

    #[init(spawn = [toggle_led, start_measurements])]
    fn init(ctx: init::Context) -> init::LateResources {
        let _p: rtic::Peripherals = ctx.core;
        let mut dp: pac::Peripherals = ctx.device;

        // Init delay timer
        let mut delay = delay::Tim7Delay::new(dp.TIM7, &mut dp.RCC);

        // Clock configuration. Use HSI at 16 MHz.
        let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());

        // TODO: Use the MSI (Multispeed Internal) clock source instead?
        // However, currently the timer cannot be initialized (at least when
        // connected through the debugger) when enabling MSI.
        // Note that using MSI will require updating the monotonic impl.
        //let mut rcc = dp.RCC.freeze(
        //    hal::rcc::Config::msi(hal::rcc::MSIRange::Range5) // ~2.097 MHz
        //);

        // Initialize monotonic timer TIM6. Use TIM6 since it has lower current
        // consumption than TIM2/3 or TIM21/22.
        Tim6Monotonic::initialize(dp.TIM6);

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
            "Booting: Gfrörli firmware={} hardware={}",
            FIRMWARE_VERSION,
            hardware_version.detect(),
        )
        .unwrap();

        // Check whether we just woke up after a panic
        if let Some(msg) = panic_persist::get_panic_message_utf8() {
            // If yes, send backtrace via serial
            writeln!(debug, "=== 🔥 FOUND PANIC 🔥 ===").ok();
            writeln!(debug, "{}", msg.trim_end()).ok();
            writeln!(debug, "==== 🚒 END PANIC 🚒 ====").ok();
        }

        // Dump EEPROM config data
        if cfg!(feature = "dev") {
            writeln!(debug, "\nEEPROM contents at 0x0808_0000:").unwrap();
            let config_data: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    config::BASE_ADDR as *const u8,
                    config::CONFIG_DATA_SIZE,
                )
            };
            for (i, byte) in config_data.iter().enumerate() {
                write!(debug, " {:02x}", byte).unwrap();
                if (i + 1) % 16 == 0 {
                    write!(debug, "\n").unwrap();
                }
            }
            write!(debug, "\n\n").unwrap();
        }

        // Read config from EEPROM
        //let config = match Config::read_from_eeprom(&mut dp.FLASH) {
        //    Ok(c) => c,
        //    Err(e) => panic!("Error: Could not read config from EEPROM: {}", e),
        //};
        //writeln!(debug, "Loaded config (v{}) from EEPROM", config.version).unwrap();

        // Initialize supply monitor
        let adc = dp.ADC.constrain(&mut rcc);
        let a1 = gpioa.pa1.into_analog();
        let adc_enable_pin = gpioa.pa5.into_push_pull_output().downgrade();
        let mut supply_monitor = SupplyMonitor::new(a1, adc, adc_enable_pin);
        let val = supply_monitor.read_supply();
        writeln!(debug, "Supply: {:?}", val).unwrap();

        writeln!(debug, "Initialize one-wire DS18B20 sensor").unwrap();
        let one_wire_pin = gpioa.pa6.into_open_drain_output();
        let mut one_wire = OneWire::new(one_wire_pin).unwrap();
        let ds18b20 =
            Ds18b20::find(&mut one_wire, &mut delay).expect("Could not find DS18B20 sensor");

        // Initialize LEDs
        writeln!(debug, "Initialize LEDs").unwrap();
        let mut status_leds = StatusLeds::new(
            gpiob.pb1.into_push_pull_output().downgrade(),
            gpiob.pb0.into_push_pull_output().downgrade(),
            gpioa.pa7.into_push_pull_output().downgrade(),
        );
        status_leds.disable_all();

        writeln!(debug, "Initialize I²C peripheral").unwrap();
        let sda = gpioa.pa10.into_open_drain_output();
        let scl = gpioa.pa9.into_open_drain_output();
        let i2c = dp.I2C1.i2c(sda, scl, 10.khz(), &mut rcc);

        let mut sht = shtc3(i2c);
        sht.wakeup(&mut delay)
            .expect("SHTCx: Could not wake up sensor");

        // Spawn tasks
        ctx.spawn.toggle_led().unwrap();
        ctx.spawn.start_measurements().unwrap();

        writeln!(debug, "Initialization done").unwrap();

        init::LateResources {
            debug,
            status_leds,
            sht,
            one_wire,
            ds18b20,
            delay,
        }
    }

    #[task(schedule = [toggle_led], resources = [status_leds])]
    fn toggle_led(ctx: toggle_led::Context) {
        static mut STATE: LedState = LedState::Green;

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

        // Re-schedule ourselves
        ctx.schedule
            .toggle_led(ctx.scheduled + 500.millis())
            .unwrap();
    }

    /// Start a measurement for both the SHTCx sensor and the DS18B20 sensor.
    #[task(resources = [delay, sht, one_wire, ds18b20], schedule = [read_measurement_results])]
    fn start_measurements(mut ctx: start_measurements::Context) {
        ctx.resources
            .sht
            .start_measurement(PowerMode::NormalMode)
            .expect("SHTCx: Failed to start measurement");
        ctx.resources
            .ds18b20
            .start_measurement(&mut ctx.resources.one_wire, ctx.resources.delay)
            .expect("DS18B20: Failed to start measurement");

        // Schedule reading of the measurement results
        ctx.schedule
            .read_measurement_results(ctx.scheduled + 500.millis())
            .unwrap();
    }

    /// Read measurement results from the sensors. Re-schedule a measurement.
    #[task(resources = [debug, delay, sht, one_wire, ds18b20], schedule = [start_measurements])]
    fn read_measurement_results(mut ctx: read_measurement_results::Context) {
        // Fetch measurement results
        let measurement = ctx
            .resources
            .sht
            .get_measurement_result()
            .expect("SHTCx: Failed to read measurement");
        let ds18b20_measurement = ctx
            .resources
            .ds18b20
            .read_raw_temperature_data(&mut ctx.resources.one_wire, ctx.resources.delay)
            .expect("DS18B20: Failed to read measurement");

        // Print results
        if cfg!(feature = "dev") {
            writeln!(
                ctx.resources.debug,
                "DS18B20: {:.2}°C (0x{:04x}) | SHTC3: {:.2}°C, {:.2}%RH",
                (ds18b20_measurement as f32) / 16.0,
                ds18b20_measurement,
                measurement.temperature.as_degrees_celsius(),
                measurement.humidity.as_percent()
            )
            .unwrap();
        } else {
            writeln!(
                ctx.resources.debug,
                "DS18B20: 0x{:04x} | SHTC3: {:.2}°C, {:.2}%RH",
                ds18b20_measurement,
                measurement.temperature.as_degrees_celsius(),
                measurement.humidity.as_percent()
            )
            .unwrap();
        }

        // Re-schedule a measurement
        ctx.schedule
            .start_measurements(ctx.scheduled + 500.millis())
            .unwrap();
    }

    // RTIC requires that free interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    extern "C" {
        fn SPI1();
        fn SPI2();
    }
};
