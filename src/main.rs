#![no_main]
#![no_std]
#![cfg(target_arch = "arm")]

use core::fmt::Write;

use one_wire_bus::OneWire;
use panic_persist as _;
use rn2xx3::{rn2483_868, ConfirmationMode, DataRateEuCn, Driver as Rn2xx3, Freq868, JoinMode};
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
use leds::StatusLeds;
use measurement::{EncodedMeasurement, MeasurementMessage, MAX_MSG_LEN, U12};
use monotonic_stm32l0::{Instant, Tim6Monotonic, U16Ext};
use supply_monitor::SupplyMonitor;
use version::HardwareVersionDetector;

const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEV_ADDR: &str = env!("GFROERLI_DEV_ADDR");
const NETWORK_SESSION_KEY: &str = env!("GFROERLI_NETWORK_SESSION_KEY");
const APP_SESSION_KEY: &str = env!("GFROERLI_APP_SESSION_KEY");

enum LedDisableTarget {
    Red,
    All,
}

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

        // RN2483
        rn: Rn2xx3<Freq868, hal::serial::Serial<pac::LPUART1>>,

        // Blocking delay provider
        delay: Tim7Delay,
    }

    #[init(spawn = [join, start_measurements])]
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
        let lpuart1 = serial::Serial::lpuart1(
            dp.LPUART1,
            gpioa.pa2.into_floating_input(),
            gpioa.pa3.into_floating_input(),
            // Config: See RN2483 datasheet, table 3-1
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
        let config = match Config::read_from_eeprom(&mut dp.FLASH) {
            Ok(c) => c,
            Err(e) => panic!("Error: Could not read config from EEPROM: {}", e),
        };
        writeln!(debug, "Loaded config (v{}) from EEPROM", config.version).unwrap();

        // Initialize supply monitor
        let adc = dp.ADC.constrain(&mut rcc);
        let a1 = gpioa.pa1.into_analog();
        let adc_enable_pin = gpioa.pa5.into_push_pull_output().downgrade();
        let mut supply_monitor = SupplyMonitor::new(a1, adc, adc_enable_pin);
        let val = supply_monitor.read_supply();
        writeln!(debug, "Supply: {:?}", val).unwrap();

        // Initialize DS18B20
        writeln!(debug, "Init DS18B20…").unwrap();
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

        // Set up I²C pins
        writeln!(debug, "Initialize I²C peripheral").unwrap();
        let sda = gpioa.pa10.into_open_drain_output();
        let scl = gpioa.pa9.into_open_drain_output();
        let i2c = dp.I2C1.i2c(sda, scl, 10.khz(), &mut rcc);

        // Initialize SHTC3
        writeln!(debug, "Init SHTC3…").unwrap();
        let mut sht = shtc3(i2c);
        sht.wakeup(&mut delay)
            .expect("SHTCx: Could not wake up sensor");

        // Initialize RN2xx3
        writeln!(debug, "Init RN2483…").unwrap();
        let mut rn = rn2483_868(lpuart1);
        writeln!(debug, "RN2483: Hard reset…").unwrap();
        let mut rn_reset_pin = gpioa.pa4.into_push_pull_output();
        rn_reset_pin.set_low().expect("Could not set RN reset pin");
        delay.delay_us(1000); // TODO: How long?
        rn_reset_pin.set_high().expect("Could not set RN reset pin");
        let version = rn.reset().expect("Could not soft reset");
        writeln!(debug, "RN2483: Version {}", version).unwrap();

        // Show device info
        writeln!(debug, "RN2483: Device info").unwrap();
        let hweui = rn.hweui().expect("Could not read hweui");
        writeln!(debug, "  Hardware EUI: {}", hweui).unwrap();
        let model = rn.model().expect("Could not read model");
        writeln!(debug, "  Model: {:?}", model).unwrap();
        let vdd = rn.vdd().expect("Could not read vdd");
        writeln!(debug, "  VDD voltage: {} mV", vdd).unwrap();

        // Set keys
        writeln!(debug, "RN2483: Setting keys...").unwrap();
        writeln!(debug, "  Dev addr: {}", DEV_ADDR).unwrap();

        rn.set_dev_addr_hex(DEV_ADDR)
            .expect("Could not set dev addr");
        rn.set_network_session_key_hex(NETWORK_SESSION_KEY)
            .expect("Could not set network session key");
        rn.set_app_session_key_hex(APP_SESSION_KEY)
            .expect("Could not set app session key");
        rn.set_data_rate(DataRateEuCn::Sf8Bw125)
            .expect("Could not set data rate");

        // Spawn tasks
        ctx.spawn.join().unwrap();
        ctx.spawn.start_measurements().unwrap();

        writeln!(debug, "Initialization done").unwrap();

        init::LateResources {
            debug,
            status_leds,
            sht,
            one_wire,
            ds18b20,
            rn,
            delay,
        }
    }

    /// Join LoRaWAN network via ABP.
    #[task(resources = [debug, status_leds, rn], schedule = [disable_led], priority = 1)]
    fn join(ctx: join::Context) {
        let debug = ctx.resources.debug;
        let mut status_leds = ctx.resources.status_leds;
        let rn = ctx.resources.rn;

        status_leds.lock(|leds: &mut StatusLeds| leds.enable_yellow());
        for i in 1..=3 {
            writeln!(debug, "RN2483: Joining via ABP (attempt {})…", i).unwrap();
            match rn.join(JoinMode::Abp) {
                Ok(()) => {
                    writeln!(debug, "RN2483: Join successful").unwrap();
                    status_leds.lock(|leds: &mut StatusLeds| leds.enable_green());
                    ctx.schedule
                        .disable_led(Instant::now() + 100.millis(), LedDisableTarget::All)
                        .ok()
                        .unwrap();
                    break;
                }
                Err(e) => {
                    writeln!(debug, "RN2483: Join failed: {:?}", e).unwrap();
                    status_leds.lock(|leds: &mut StatusLeds| leds.enable_red());
                    ctx.schedule
                        .disable_led(Instant::now() + 100.millis(), LedDisableTarget::Red)
                        .ok()
                        .unwrap();
                }
            }
        }
    }

    #[task(resources = [status_leds], capacity = 2, priority = 2)]
    fn disable_led(ctx: disable_led::Context, target: LedDisableTarget) {
        match target {
            LedDisableTarget::Red => ctx.resources.status_leds.disable_red(),
            LedDisableTarget::All => ctx.resources.status_leds.disable_all(),
        }
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
            .read_measurement_results(Instant::now() + 500.millis())
            .unwrap();
    }

    /// Read measurement results from the sensors. Re-schedule a measurement.
    #[task(resources = [debug, delay, sht, one_wire, ds18b20, rn], schedule = [start_measurements])]
    fn read_measurement_results(mut ctx: read_measurement_results::Context) {
        static mut COUNTER: usize = 0;

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

        // For testing, transmit every 30s
        if *COUNTER % 30 == 0 {
            let fport = 123;

            // Encode measurement
            let message = MeasurementMessage {
                t_water: Some(U12::new(ds18b20_measurement)),
                t_inside: Some(0b1100_0011_1010_0101),
                rh_inside: Some(0b0011_1100_0101_1010),
                v_supply: Some(U12::new(0b1111_1010_0101)),
            };
            let mut buf = EncodedMeasurement([0u8; MAX_MSG_LEN]);
            let length = message.encode(&mut buf);

            // Transmit
            writeln!(ctx.resources.debug, "Transmitting measurement...").unwrap();
            let tx_result = ctx.resources.rn.transmit_slice(
                ConfirmationMode::Unconfirmed,
                fport,
                &buf.0[0..length],
            );
            if let Err(e) = tx_result {
                writeln!(
                    ctx.resources.debug,
                    "Error: Transmitting LoRaWAN package failed: {:?}",
                    e
                )
                .unwrap();
            }
        }
        *COUNTER += 1;

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
