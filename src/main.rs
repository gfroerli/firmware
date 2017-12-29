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


fn main() {
    hprintln!("Hello, world!");

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
