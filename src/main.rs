#![no_main]
#![no_std]

extern crate panic_halt;

use core::fmt::Write;

use rtfm::app;
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{self as hal, pac, serial, time};

mod leds;

use leds::{LedState, StatusLeds};

#[app(device = stm32l0::stm32l0x1, peripherals = true)]
const APP: () = {
    struct Resources {
        /// Serial debug output
        debug: hal::serial::Serial<pac::USART1>,

        status_leds: StatusLeds,
        timer: hal::timer::Timer<pac::TIM6>,
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

        // Initialize timer to blink LEDs. Use TIM6 since it has lower current
        // consumption than TIM2/3 or TIM21/22.
        let mut timer = hal::timer::Timer::tim6(dp.TIM6, 2.hz(), &mut rcc);
        timer.listen();

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

        writeln!(debug, "Greetings, Rusty world, from Gfrörli v2!").unwrap();
        writeln!(debug, "Debug output initialized on USART1!").unwrap();

        // Initialize LEDs
        let mut status_leds = StatusLeds::new(
            gpiob.pb1.into_push_pull_output().downgrade(),
            gpiob.pb0.into_push_pull_output().downgrade(),
            gpioa.pa7.into_push_pull_output().downgrade(),
        );
        status_leds.disable_all();

        writeln!(debug, "Initialization done").unwrap();

        init::LateResources {
            debug,
            status_leds,
            timer,
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

    // RTFM requires that free interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    extern "C" {
        fn SPI1();
        fn SPI2();
    }
};
