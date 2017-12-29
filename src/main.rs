//! Prints "Hello, world!" on the OpenOCD console using semihosting
#![feature(used)]
#![no_std]

#[macro_use]
extern crate cortex_m;
extern crate cortex_m_rt;
extern crate lpc11uxx;

mod leds;

use cortex_m::asm;
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

fn clock_setup() {
    unsafe {
        let syscon = &*lpc11uxx::SYSCON.get();

        syscon.pdruncfg.modify(|_,w| w.sysosc_pd().powered());
        syscon.sysoscctrl.write(|w| w.bits(SYSOSCCTRL_VAL));
        sleep(200);

        syscon.syspllclksel.write(|w| w.bits(SYSPLLCLKSEL_VAL));
        syscon.syspllclkuen.write(|w| w.bits(0x01));
        syscon.syspllclkuen.write(|w| w.bits(0x00));
        syscon.syspllclkuen.write(|w| w.bits(0x01));
        // wait until updated
        while syscon.syspllclkuen.read().bits() & 0x01 == 0 {
        }

        syscon.syspllctrl.write(|w| w.bits(SYSPLLCTRL_VAL));
        syscon.pdruncfg.modify(|_,w| w.syspll_pd().powered());


        syscon.mainclksel.write(|w| w.bits(MAINCLKSEL_VAL));
        syscon.mainclkuen.write(|w| w.bits(0x01));
        syscon.mainclkuen.write(|w| w.bits(0x00));
        syscon.mainclkuen.write(|w| w.bits(0x01));
        // wait until updated
        while syscon.mainclkuen.read().bits() & 0x01 == 0 {
        }
        syscon.sysahbclkdiv.write(|w| w.bits(SYSAHBCLKDIV_VAL));

        syscon.pdruncfg.modify(|_,w| w
            .usbpad_pd().usb_transceiver_poweered_down()
            .usbpll_pd().powered_down()
        );

        syscon.sysahbclkctrl.modify(|_,w| w.iocon().enable());
    }
}

fn main() {
    hprintln!("Hello, world!");
    clock_setup();

    let syscon = lpc11uxx::SYSCON.get();

    unsafe {
        // Enable GPIO clock
        hprintln!("SYSAHBCLKCTRL: {:#b}", (*syscon).sysahbclkctrl.read().bits());
        (*syscon).sysahbclkctrl.write(|w| { w.gpio().enable(); w });
        hprintln!("SYSAHBCLKCTRL: {:#b}", (*syscon).sysahbclkctrl.read().bits());
    }

    let mut leds = Leds::init();

    leds.all();
    sleep(5000);
    leds.none();
    sleep(2500);

    let delay = 1000;
    hprintln!("Starting main loop");
    loop {
        leds.on(Color::Red);
        sleep(delay);
        leds.on(Color::Yellow);
        sleep(delay);
        leds.on(Color::Green);
        sleep(delay);

        leds.off(Color::Red);
        sleep(delay);
        leds.off(Color::Yellow);
        sleep(delay);
        leds.off(Color::Green);
        sleep(delay);
    }
}

// As we are not using interrupts, we just register a dummy catch all handler
#[allow(dead_code)]
#[used]
#[link_section = ".rodata.interrupts"]
static INTERRUPTS: [extern "C" fn(); 240] = [default_handler; 240];

extern "C" fn default_handler() {
    asm::bkpt();
}
