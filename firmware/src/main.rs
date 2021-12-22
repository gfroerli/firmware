#![no_main]
#![no_std]
#![cfg(target_arch = "arm")]

// Libcore
use core::fmt::Write;

// Third party
use embedded_time::rate::{Baud, Extensions};
use one_wire_bus::OneWire;
use panic_persist as _;
use rn2xx3::{rn2483_868, ConfirmationMode, DataRateEuCn, Driver as Rn2xx3, Freq868, JoinMode};
use rtic::app;
use shtcx::{shtc3, LowPower, PowerMode, ShtC3};
use stm32l0xx_hal::gpio::{
    gpioa::{PA10, PA6, PA9},
    OpenDrain, Output,
};
use stm32l0xx_hal::{
    self as hal,
    i2c::I2c,
    pac,
    prelude::*,
    pwr,
    rtc::{Datelike, Rtc, Timelike},
    serial,
};

// First party crates
use gfroerli_common::config::{self, Config};
use gfroerli_common::measurement::{EncodedMeasurement, MeasurementMessage, MAX_MSG_LEN, U12};

// Modules
mod delay;
mod ds18b20;
mod leds;
mod monotonic_stm32l0;
mod rtc;
mod supply_monitor;
mod version;

// Crate-internal
use delay::Tim7Delay;
use ds18b20::Ds18b20;
use leds::StatusLeds;
use monotonic_stm32l0::{Instant, Tim6Monotonic, U16Ext};
use supply_monitor::SupplyMonitor;
use version::HardwareVersionDetector;

type I2C1 = I2c<pac::I2C1, PA10<Output<OpenDrain>>, PA9<Output<OpenDrain>>>;

const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Copy, Clone)]
/// Keep track which sensors should be measured
pub struct MeasurementPlan {
    measure_sht: bool,
    measure_ds18b20: bool,
    measure_voltage: bool,
}

impl MeasurementPlan {
    fn should_transmit(self) -> bool {
        self.measure_sht || self.measure_ds18b20 || self.measure_voltage
    }
}

fn bool_to_emoji(val: bool) -> &'static str {
    if val {
        "✅"
    } else {
        "❌"
    }
}

#[app(
    device = stm32l0xx_hal::pac,
    peripherals = true,
    monotonic = crate::monotonic_stm32l0::Tim6Monotonic,
    )]
const APP: () = {
    struct Resources {
        // Serial debug output
        debug: hal::serial::Serial<pac::USART1>,

        // Base measurement plan, based purely on wakeup cycle and config
        base_measurement_plan: MeasurementPlan,

        // Status LEDs
        status_leds: StatusLeds,

        // SHT temperature/humidity sensor
        sht: ShtC3<I2C1>,

        // DS18B20 water temperature sensor
        one_wire: OneWire<PA6<Output<OpenDrain>>>,
        ds18b20: Option<Ds18b20>,

        // RN2483
        rn: Rn2xx3<Freq868, hal::serial::Serial<pac::LPUART1>>,

        // Supply voltage monitor
        supply_monitor: SupplyMonitor,

        // Blocking delay provider
        delay: Tim7Delay,

        // Real-time clock
        rtc: Rtc,
    }

    #[init(spawn = [start_measurements], schedule = [disable_leds])]
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

        // Get access to PWR peripheral
        let pwr = pwr::PWR::new(dp.PWR, &mut rcc);

        // Instantiate RTC peripheral
        let mut rtc = Rtc::new(dp.RTC, &mut rcc, &pwr, None).unwrap(); // Cannot fail, since no `init` value is passed in

        // Get access to GPIOs
        let gpioa = dp.GPIOA.split(&mut rcc);
        let gpiob = dp.GPIOB.split(&mut rcc);

        // Initialize serial port(s)
        let mut debug = serial::Serial::usart1(
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
        let mut lpuart1 = serial::Serial::lpuart1(
            dp.LPUART1,
            gpioa.pa2.into_floating_input(),
            gpioa.pa3.into_floating_input(),
            // Config: See RN2483 datasheet, table 3-1
            serial::Config {
                baudrate: Baud(57_600),
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
            "\n🚀 Booting: Gfrörli firmware={} hardware={}",
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
                    writeln!(debug).unwrap();
                }
            }
            write!(debug, "\n\n").unwrap();
        }

        // Read config from EEPROM
        let config = match {
            // Note: We need to guarantee that no part of the code can write to
            // EEPROM while it's being read. To ensure that, we hold a mutable
            // reference to the FLASH peripheral.
            let _flash = &mut dp.FLASH;

            // Note(unsafe): Read with no side effects. This is fine as long as
            // the data in EEPROM is not being written while it's being read.
            // This is safe as long as we hold a mutable reference to the flash
            // peripheral. See comment above for details.
            let config_data: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    config::BASE_ADDR as *const u8,
                    config::CONFIG_DATA_SIZE,
                )
            };

            Config::from_slice(config_data)
        } {
            Ok(c) => c,
            Err(e) => panic!("Error: Could not read config from EEPROM: {}", e),
        };
        writeln!(debug, "🔧 Loaded config (v{}) from EEPROM", config.version).unwrap();
        if cfg!(feature = "dev") {
            writeln!(debug, "Config: {:?}", config).unwrap();
        }

        // Measure current time to determine the wakeup cycle
        let now = rtc.now();
        writeln!(
            debug,
            "📅 RTC Date: {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
        )
        .unwrap();
        let uptime = rtc::datetime_to_uptime(now);
        let wakeup_cycle = uptime / config.wakeup_interval_seconds as u32;
        writeln!(
            debug,
            "⏰ Uptime: {}s / Wakeup Interval: {}s / Wakeup Cycle: {}",
            uptime,
            config.wakeup_interval_seconds,
            wakeup_cycle + 1,
        )
        .unwrap();

        // End of header
        writeln!(debug).unwrap();

        // Determine measurement plan
        let measure_temp_humi = wakeup_cycle % (config.nth_temp_humi as u32) == 0;
        let measure_voltage = wakeup_cycle % (config.nth_voltage as u32) == 0;
        let measurement_plan = MeasurementPlan {
            measure_sht: measure_temp_humi,
            measure_ds18b20: measure_temp_humi,
            measure_voltage,
        };
        writeln!(
            debug,
            "Config:\n  nth_temp_humi = {}\n  nth_voltage = {}",
            config.nth_temp_humi, config.nth_voltage,
        )
        .unwrap();
        writeln!(
            debug,
            "Base measurement plan:\n  {} SHT\n  {} DS18B20\n  {} VCC\n",
            bool_to_emoji(measurement_plan.measure_sht),
            bool_to_emoji(measurement_plan.measure_ds18b20),
            bool_to_emoji(measurement_plan.measure_voltage),
        )
        .unwrap();

        // Initialize supply monitor
        let adc = dp.ADC.constrain(&mut rcc);
        let a1 = gpioa.pa1.into_analog();
        let adc_enable_pin = gpioa.pa5.into_push_pull_output().downgrade();
        let supply_monitor = SupplyMonitor::new(a1, adc, adc_enable_pin);

        // Initialize DS18B20
        writeln!(debug, "Init DS18B20…").unwrap();
        let one_wire_pin = gpioa.pa6.into_open_drain_output();
        let mut one_wire = OneWire::new(one_wire_pin).unwrap();
        let ds18b20 = Ds18b20::find(&mut one_wire, &mut delay)
            .map_err(|err| writeln!(debug, "Could not find DS18B20: {:?}", err).unwrap())
            .ok();

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
        let i2c = dp.I2C1.i2c(sda, scl, 10_000.Hz(), &mut rcc);

        // Initialize SHTC3
        writeln!(debug, "Init SHTC3…").unwrap();
        let mut sht = shtc3(i2c);
        sht.wakeup(&mut delay).unwrap_or_else(|e| {
            writeln!(debug, "SHTCx: Could not wake up sensor: {:?}", e).unwrap()
        });

        // Reset RN2xx3
        writeln!(debug, "Init RN2483…").unwrap();
        writeln!(debug, "RN2483: Hard reset…").unwrap();
        let mut rn_reset_pin = gpioa.pa4.into_push_pull_output();
        rn_reset_pin.set_low().expect("Could not set RN reset pin");
        delay.delay_us(500);
        rn_reset_pin.set_high().expect("Could not set RN reset pin");
        delay.delay_ms(195); // 100ms until TX line is up, 85ms until version is sent, 10ms extra

        // Clear hardware error flags
        lpuart1.clear_errors();

        // Initialize RN2xx3
        let mut rn = rn2483_868(lpuart1);

        // Show device info
        writeln!(debug, "RN2483: Device info").unwrap();
        match rn.hweui() {
            Ok(hweui) => writeln!(debug, "  Hardware EUI: {}", hweui).unwrap(),
            Err(e) => writeln!(debug, "  Could not read hweui: {:?}", e).unwrap(),
        };
        match rn.version() {
            Ok(version) => writeln!(debug, "  Version: {}", version).unwrap(),
            Err(e) => writeln!(debug, "  Could not read version: {:?}", e).unwrap(),
        };
        match rn.vdd() {
            Ok(vdd) => writeln!(debug, "  VDD voltage: {} mV", vdd).unwrap(),
            Err(e) => writeln!(debug, "  Could not read voltage: {:?}", e).unwrap(),
        };

        // Print device address
        writeln!(debug, "RN2483: Setting keys...").unwrap();
        writeln!(
            debug,
            "  Configured dev addr: {:x}{:x}{:x}{:x}",
            config.devaddr[0], config.devaddr[1], config.devaddr[2], config.devaddr[3],
        )
        .unwrap();

        // Check whether credentials have already been configured. Do this by
        // looking at the device address. If it's still the default value
        // (0x00000000), or if it's configured with a different device address,
        // then set new keys. Otherwise, we can reuse the existing keys.
        //
        // (Note: This assumes that the keys for a device with a specific
        // address do not change. If you want to change the keys, you must also
        // change the device address.)
        //
        // TODO: Right now, when unplugging and re-plugging the devboard, often
        // the dev addr is lost even though it was stored in EEPROM. This would
        // be bad since we would lose the frame counter. Test whether this
        // happens on controlled shutdowns (in standby mode) as well.
        let dev_addr = rn.get_dev_addr_slice().unwrap();
        writeln!(
            debug,
            "  Current dev addr: {:02x}{:02x}{:02x}{:02x}",
            dev_addr[0], dev_addr[1], dev_addr[2], dev_addr[3],
        )
        .unwrap();
        let upctr = rn.get_upctr().unwrap();
        writeln!(debug, "  Current up counter: {}", upctr).unwrap();
        if dev_addr == config.devaddr {
            writeln!(debug, "  Re-using previously stored credentials").unwrap();
        } else {
            // Enable status LED to show that joining is in progress
            status_leds.enable_yellow();

            // Set keys and counters
            writeln!(
                debug,
                "  Stored device address does not match, settings new keys"
            )
            .unwrap();
            rn.set_dev_addr_slice(&config.devaddr)
                .expect("Could not set dev addr");
            rn.set_app_session_key_slice(&config.appskey)
                .expect("Could not set app session key");
            rn.set_network_session_key_slice(&config.nwkskey)
                .expect("Could not set network session key");
            rn.set_data_rate(DataRateEuCn::Sf8Bw125)
                .expect("Could not set data rate");
            rn.set_upctr(0).expect("Could not set up counter");
            rn.set_dnctr(0).expect("Could not set down counter");
        }

        // Join LoRaWAN network via ABP (this should be instantaneous)
        match rn.join(JoinMode::Abp) {
            Ok(()) => {
                writeln!(debug, "RN2483: Join successful").unwrap();
                status_leds.enable_green();
                ctx.schedule
                    .disable_leds(Instant::now() + 100.millis())
                    .ok()
                    .unwrap();
            }
            Err(e) => {
                writeln!(debug, "RN2483: Join failed: {:?}", e).unwrap();
                status_leds.enable_red();
                ctx.schedule
                    .disable_leds(Instant::now() + 1000.millis())
                    .ok()
                    .unwrap();
            }
        }

        // Spawn tasks
        ctx.spawn.start_measurements().unwrap();

        writeln!(debug, "Initialization done").unwrap();

        init::LateResources {
            debug,
            base_measurement_plan: measurement_plan,
            status_leds,
            sht,
            one_wire,
            ds18b20,
            rn,
            supply_monitor,
            delay,
            rtc,
        }
    }

    /// Disable all LEDs.
    #[task(resources = [status_leds], capacity = 2, priority = 2)]
    fn disable_leds(ctx: disable_leds::Context) {
        ctx.resources.status_leds.disable_all();
    }

    /// Start a measurement for both the SHTCx sensor and the DS18B20 sensor.
    #[task(resources = [base_measurement_plan, delay, sht, one_wire, ds18b20], schedule = [read_measurement_results])]
    fn start_measurements(ctx: start_measurements::Context) {
        let mut measurement_plan = *ctx.resources.base_measurement_plan;
        ctx.resources
            .sht
            .start_measurement(PowerMode::NormalMode)
            .unwrap_or_else(|_| measurement_plan.measure_sht = false);
        if let Some(ds18b20) = ctx.resources.ds18b20 {
            ds18b20
                .start_measurement(ctx.resources.one_wire, ctx.resources.delay)
                .unwrap_or_else(|_| measurement_plan.measure_ds18b20 = false);
        } else {
            measurement_plan.measure_ds18b20 = false;
        }

        // Schedule reading of the measurement results
        ctx.schedule
            .read_measurement_results(Instant::now() + 500.millis(), measurement_plan)
            .unwrap();
    }

    /// Read measurement results from the sensors. Re-schedule a measurement.
    #[task(resources = [debug, delay, sht, one_wire, ds18b20, supply_monitor, rn], schedule = [start_measurements])]
    fn read_measurement_results(
        ctx: read_measurement_results::Context,
        measurement_plan: MeasurementPlan,
    ) {
        // Fetch measurement results
        let sht_measurement = if measurement_plan.measure_sht {
            ctx.resources.sht.get_raw_measurement_result().ok()
        } else {
            None
        };
        let shtc3_temperature = sht_measurement.as_ref().map(|v| v.temperature);
        let shtc3_humidity = sht_measurement.as_ref().map(|v| v.humidity);
        let ds18b20_measurement = if measurement_plan.measure_ds18b20 {
            ctx.resources.ds18b20.and_then(|ds18b20| {
                ds18b20
                    .read_raw_temperature_data(ctx.resources.one_wire, ctx.resources.delay)
                    .ok()
            })
        } else {
            None
        };

        // Measure current supply voltage
        let v_supply = ctx.resources.supply_monitor.read_supply_raw_u12();

        // Print results
        let mut first = true;
        macro_rules! delimit {
            () => {{
                #[allow(unused_assignments)]
                // https://github.com/rust-lang/rust/issues/85774#issuecomment-857105289
                if !first {
                    write!(ctx.resources.debug, " | ").unwrap();
                } else {
                    first = false;
                }
            }};
        }

        if cfg!(feature = "dev") {
            // Development mode, print human-readable information
            use shtcx::{Humidity, Temperature};
            if let Some(ds18b20) = ds18b20_measurement {
                delimit!();
                write!(
                    ctx.resources.debug,
                    "DS18B20: {:.2}°C (0x{:04x})",
                    (ds18b20 as f32) / 16.0,
                    ds18b20,
                )
                .unwrap();
            }
            if let Some(sht) = sht_measurement {
                delimit!();
                write!(
                    ctx.resources.debug,
                    "SHTC3: {:.2}°C, {:.2}%RH",
                    Temperature::from_raw(sht.temperature).as_degrees_celsius(),
                    Humidity::from_raw(sht.humidity).as_percent(),
                )
                .unwrap();
            }
            if let Some(v_supply_f32) = v_supply
                .as_ref()
                .map(U12::as_u16)
                .map(SupplyMonitor::convert_input)
            {
                delimit!();
                write!(ctx.resources.debug, "VDD: {:.3}V", v_supply_f32).unwrap();
            }
        } else {
            // Production mode, print raw values directly
            if let Some(ds18b20) = ds18b20_measurement {
                delimit!();
                write!(ctx.resources.debug, "DS18B20: 0x{:04x}", ds18b20).unwrap();
            }
            if let Some(sht) = sht_measurement {
                delimit!();
                write!(
                    ctx.resources.debug,
                    "SHTC3: 0x{:04x}, 0x{:04x}",
                    sht.temperature, sht.humidity
                )
                .unwrap();
            }
            if let Some(v_supply_u12) = v_supply {
                delimit!();
                write!(ctx.resources.debug, "VDD: 0x{:04x}", v_supply_u12.as_u16(),).unwrap();
            }
        }
        writeln!(ctx.resources.debug).unwrap();

        if measurement_plan.should_transmit() {
            let fport = 123;

            // Encode measurement
            let message = MeasurementMessage {
                t_water: ds18b20_measurement.map(U12::new),
                t_inside: shtc3_temperature,
                rh_inside: shtc3_humidity,
                v_supply,
            };
            let mut buf = EncodedMeasurement([0u8; MAX_MSG_LEN]);
            let length = message.encode(&mut buf);

            // Transmit
            writeln!(ctx.resources.debug, "📣 Transmitting measurement...").unwrap();
            let tx_result = ctx.resources.rn.transmit_slice(
                ConfirmationMode::Unconfirmed,
                fport,
                &buf.0[0..length],
            );
            if let Err(e) = &tx_result {
                writeln!(
                    ctx.resources.debug,
                    "Error: Transmitting LoRaWAN package failed: {:?}",
                    e
                )
                .unwrap();
            }
            if let Ok(Some(downlink)) = tx_result {
                writeln!(ctx.resources.debug, "Downlink: {:?}", downlink).unwrap();
            }
        }

        // Persist MAC changes
        writeln!(ctx.resources.debug, "RN2483: Calling \"mac save\"").unwrap();
        ctx.resources.rn.save_config().unwrap_or_else(|e| {
            writeln!(
                ctx.resources.debug,
                "RN2483: Could not save config: {:?}",
                e
            )
            .unwrap();
        });

        // Re-schedule a measurement
        // (TODO: Go to sleep here. For now, simulate 10s of sleep.)
        ctx.resources.delay.delay_ms(3000);
        ctx.resources.delay.delay_ms(3000);
        ctx.schedule
            .start_measurements(ctx.scheduled + 4000.millis())
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
