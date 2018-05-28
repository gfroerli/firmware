//! Prints "Hello, world!" on the OpenOCD console using semihosting
#![feature(used, lang_items)]
#![no_main]
#![no_std]

extern crate cortex_m;
#[macro_use(entry, exception)]
extern crate cortex_m_rt;
extern crate lpc11uxx;
extern crate panic_semihosting;

mod leds;

use lpc11uxx::{Peripherals, SYSCON};

use cortex_m::asm;
use cortex_m_rt::ExceptionFrame;
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
        syscon.sysahbclkctrl.modify(|_,w| w.iocon().enable());
    }
}

entry!(main);
fn main() -> ! {
    let p = Peripherals::take().unwrap();
    let mut syscon = p.SYSCON;
    let mut iocon = p.IOCON;
    let mut gpio = p.GPIO_PORT;

    //hprintln!("Hello, world!");
    clock_setup(&mut syscon);

    // Enable GPIO clock
    //hprintln!("SYSAHBCLKCTRL: {:#b}", (*syscon).sysahbclkctrl.read().bits());
    (*syscon).sysahbclkctrl.write(|w| { w.gpio().enable(); w });
    //hprintln!("SYSAHBCLKCTRL: {:#b}", (*syscon).sysahbclkctrl.read().bits());

    let mut leds = Leds::init(&mut iocon, &mut gpio);

    leds.all(&mut gpio);
    sleep(5000);
    leds.none(&mut gpio);
    sleep(2500);

    let delay = 5000;
    //hprintln!("Starting main loop");
    loop {
        leds.on(&mut gpio, Color::Red);
        sleep(delay);
        leds.on(&mut gpio, Color::Yellow);
        sleep(delay);
        leds.on(&mut gpio, Color::Green);
        sleep(delay);

        leds.off(&mut gpio, Color::Red);
        sleep(delay);
        leds.off(&mut gpio, Color::Yellow);
        sleep(delay);
        leds.off(&mut gpio, Color::Green);
        sleep(delay);
    }
}

exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

