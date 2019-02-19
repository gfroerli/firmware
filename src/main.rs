//! Prints "Hello, world!" on the OpenOCD console using semihosting
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt;
extern crate cortex_m_semihosting as sh;
extern crate lpc11uxx_hal;
extern crate panic_semihosting;
extern crate embedded_hal;

mod leds;

use core::fmt::Write;

use lpc11uxx_hal::delay::Delay;
use lpc11uxx_hal::lpc11uxx;
use lpc11uxx::{CorePeripherals, Peripherals, SYSCON};

use embedded_hal::blocking::delay::DelayMs;

use cortex_m::asm;
use cortex_m_rt::entry;
use sh::hio;
use leds::{Leds, Color};


/// Busy-loop the specified number of cycles.
fn sleep(cycles: u32) {
    for _ in 0..cycles {
        asm::nop()
    }
}

const SYSOSCCTRL_VAL: u32 = 0x00000000; // Reset: 0x000
const SYSPLLCTRL_VAL: u32 = 0x00000023; // Reset: 0x000
const SYSPLLCLKSEL_VAL: u32 = 0x00000001; // Reset: 0x000
const MAINCLKSEL_VAL: u32 = 0x00000003; // Reset: 0x000
const SYSAHBCLKDIV_VAL: u32 = 0x00000001; // Reset: 0x001

/// Configure clock for external 12MHz crystal
fn clock_setup(syscon: &mut SYSCON) {
    unsafe {

        // Power-up system oscillator
        syscon.pdruncfg.modify(|_,w| w.sysosc_pd().powered());
        syscon.sysoscctrl.write(|w| w.bits(SYSOSCCTRL_VAL));
        sleep(200);

        // select PLL input
        syscon.syspllclksel.write(|w| w.bits(SYSPLLCLKSEL_VAL));
        // update clock source
        syscon.syspllclkuen.write(|w| w.bits(0x01));
        // toggle update register
        syscon.syspllclkuen.write(|w| w.bits(0x00));
        syscon.syspllclkuen.write(|w| w.bits(0x01));
        // wait until updated
        while syscon.syspllclkuen.read().bits() & 0x01 == 0 {
        }

        // power-up SYSPLL
        syscon.syspllctrl.write(|w| w.bits(SYSPLLCTRL_VAL));
        syscon.pdruncfg.modify(|_,w| w.syspll_pd().powered());


        // select PLL clock output
        syscon.mainclksel.write(|w| w.bits(MAINCLKSEL_VAL));
        // update MCLK clock source
        syscon.mainclkuen.write(|w| w.bits(0x01));
        // toggle update register
        syscon.mainclkuen.write(|w| w.bits(0x00));
        syscon.mainclkuen.write(|w| w.bits(0x01));
        // wait until updated
        while syscon.mainclkuen.read().bits() & 0x01 == 0 {
        }
        syscon.sysahbclkdiv.write(|w| w.bits(SYSAHBCLKDIV_VAL));

        // power-down USB PHY and PLL
        syscon.pdruncfg.modify(|_,w| w
            .usbpad_pd().usb_transceiver_poweered_down()
            .usbpll_pd().powered_down()
        );

        // System clock to the IOCON needs to be enabled for most of the I/O
        // related peripherals to work
        syscon.sysahbclkctrl.modify(|_,w| w.iocon().enabled());
    }
}

#[entry]
fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();
    let p = Peripherals::take().unwrap();
    let cp = CorePeripherals::take().unwrap();
    let mut syscon = p.SYSCON;
    let mut iocon = p.IOCON;
    let mut gpio = p.GPIO_PORT;

    writeln!(stdout, "Hello, world!").unwrap();
    clock_setup(&mut syscon);

    let mut delay = Delay::new(cp.SYST, 48_000_000);

    // Enable GPIO clock
    writeln!(stdout, "SYSAHBCLKCTRL: {:#b}", (*syscon).sysahbclkctrl.read().bits()).unwrap();
    (*syscon).sysahbclkctrl.write(|w| { w.gpio().enabled(); w });
    writeln!(stdout, "SYSAHBCLKCTRL: {:#b}", (*syscon).sysahbclkctrl.read().bits()).unwrap();

    let mut leds = Leds::init(&mut iocon, &mut gpio);

    writeln!(stdout, "Initialized").unwrap();

    leds.all(&mut gpio);
    delay.delay_ms(1000);
    leds.none(&mut gpio);
    delay.delay_ms(1000);

    let delay_ms = 1000;
    writeln!(stdout, "Starting main loop").unwrap();
    loop {
        leds.on(&mut gpio, Color::Red);
        delay.delay_ms(delay_ms);
        leds.on(&mut gpio, Color::Yellow);
        delay.delay_ms(delay_ms);
        leds.on(&mut gpio, Color::Green);
        delay.delay_ms(delay_ms);

        leds.off(&mut gpio, Color::Red);
        delay.delay_ms(delay_ms);
        leds.off(&mut gpio, Color::Yellow);
        delay.delay_ms(delay_ms);
        leds.off(&mut gpio, Color::Green);
        delay.delay_ms(delay_ms);
    }
}
