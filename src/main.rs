#![no_main]
#![no_std]

extern crate panic_halt;

use core::fmt::{Write, Debug};

use rtic::app;
use shtcx::{shtc3, LowPower, PowerMode, ShtC3};
use stm32l0::stm32l0x1::I2C1;
use stm32l0xx_hal::gpio::{
    gpioa::{PA10, PA9},
    OpenDrain, Output,
};
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{self as hal, i2c::I2c, pac, serial, time};

use embedded_hal::blocking::delay::{DelayUs, DelayMs};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use one_wire_bus::{OneWire, OneWireResult};

use ds18b20::{self, Ds18b20, Resolution};

fn find_devices<P, E>(
    delay: &mut impl DelayUs<u16>,
    tx: &mut impl Write,
    one_wire_pin: P,
) -> OneWire<P>
    where
        P: OutputPin<Error=E> + InputPin<Error=E>,
        E: Debug
{
    let mut one_wire_bus = OneWire::new(one_wire_pin).unwrap();
    for device_address in one_wire_bus.devices(false, delay) {
        // The search could fail at any time, so check each result. The iterator automatically
        // ends after an error.
        let device_address = device_address.unwrap();

        // The family code can be used to identify the type of device
        // If supported, another crate can be used to interact with that device at the given address
        writeln!(tx, "Found device at address {:?} with family code: {:#x?}",
                 device_address, device_address.family_code()).unwrap();
    }
    one_wire_bus
}

fn get_temperature<P, E>(
    delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    tx: &mut impl Write,
    one_wire_bus: &mut OneWire<P>,
) -> OneWireResult<(), E>
    where
        P: OutputPin<Error=E> + InputPin<Error=E>,
        E: Debug
{
    // initiate a temperature measurement for all connected devices
    ds18b20::start_simultaneous_temp_measurement(one_wire_bus, delay)?;

    // wait until the measurement is done. This depends on the resolution you specified
    // If you don't know the resolution, you can obtain it from reading the sensor data,
    // or just wait the longest time, which is the 12-bit resolution (750ms)
    Resolution::Bits12.delay_for_measurement_time(delay);

    // iterate over all the devices, and report their temperature
    let mut search_state = None;
    loop {
        if let Some((device_address, state)) = one_wire_bus.device_search(search_state.as_ref(), false, delay)? {
            search_state = Some(state);
            if device_address.family_code() != ds18b20::FAMILY_CODE {
                // skip other devices
                continue;
            }
            // You will generally create the sensor once, and save it for later
            let sensor = Ds18b20::new(device_address)?;

            // contains the read temperature, as well as config info such as the resolution used
            let sensor_data = sensor.read_data(one_wire_bus, delay)?;
            writeln!(tx, "Device at {:?} is {}°C", device_address, sensor_data.temperature).ok();
        } else {
            break;
        }
    }
    Ok(())
}


mod leds;
mod version;

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
        sht_timer: hal::timer::Timer<pac::TIM3>,
        sht: ShtC3<I2c<I2C1, PA10<Output<OpenDrain>>, PA9<Output<OpenDrain>>>>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let p: cortex_m::Peripherals = ctx.core;
        let dp: pac::Peripherals = ctx.device;

        // Clock configuration. Use HSI at 16 MHz.
        let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi16());
        let syst = p.SYST;
        let mut delay = hal::delay::Delay::new(syst, rcc.clocks);

        // TODO: Use the MSI (Multispeed Internal) clock source instead.
        // However, currently the timer cannot be initialized (at least when
        // connected through the debugger) when enabling MSI.
        //let mut rcc = dp.RCC.freeze(
        //    hal::rcc::Config::msi(hal::rcc::MSIRange::Range5) // ~2.097 MHz
        //);

        // Initialize timer to blink LEDs. Use TIM6 since it has lower current
        // consumption than TIM2/3 or TIM21/22.
        let mut timer = hal::timer::Timer::tim6(dp.TIM6, 2.hz(), &mut rcc);
        timer.listen();

        let mut sht_timer = hal::timer::Timer::tim3(dp.TIM3, 2.hz(), &mut rcc);
        sht_timer.listen();

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
        let mut one_wire_bus = find_devices(&mut delay, &mut debug, one_wire_pin);
        get_temperature(&mut delay, &mut debug, &mut one_wire_bus).ok();


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
            sht_timer,
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

    #[task(binds = TIM3, resources = [sht_timer, sht, debug])]
    fn sht_timer(ctx: sht_timer::Context) {
        static mut STATE: ShtState = ShtState::StartMeasurement;

        // Clear the interrupt flag
        ctx.resources.sht_timer.clear_irq();

        *STATE = match STATE {
            ShtState::StartMeasurement => {
                ctx.resources
                    .sht
                    .start_measurement(PowerMode::NormalMode)
                    .expect("SHTCx: Failed to start measurement");
                ShtState::ReadResults
            }
            ShtState::ReadResults => {
                let measurement = ctx
                    .resources
                    .sht
                    .get_measurement_result()
                    .expect("SHTCx: Failed to read measurement");
                writeln!(
                    ctx.resources.debug,
                    " {:.2} °C, {:.2} %RH",
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
