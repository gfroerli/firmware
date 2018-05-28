//! Control the debug LEDs on the PCB.

use lpc11uxx::{IOCON, GPIO_PORT};

/// The colors of the debug LEDs.
pub enum Color {
    /// The red LED
    Red,
    /// The yellow LED
    Yellow,
    /// The green LED
    Green,
}

pub struct Leds;

impl Leds {
    /// Initialize the LEDs.
    ///
    /// * Set pin functions (GPIO, no pull up/down)
    /// * Set pin direction (output)
    pub fn init(iocon: &mut IOCON, gpio: &mut GPIO_PORT) -> Self {

        unsafe {

            // Set port functions
            (*iocon).pio1_22.write(|w| w
                .func().pio1_22_()
                .mode().inactive_no_pull_do());
            (*iocon).pio0_17.write(|w| w
                .func().pio0_17_()
                .mode().inactive_no_pull_do());
            (*iocon).pio1_16.write(|w| w
                .func().pio1_16_()
                .mode().inactive_no_pull_do());

            // Set pin directions to output
            (*gpio).dir[1].modify(|r, w| w.bits(r.bits() | (1<<16) | (1<<22)));
            (*gpio).dir[0].modify(|r, w| w.bits(r.bits() | (1<<17)));

        }

        Leds { }
    }

    /// Enable the specified LED.
    pub fn on(&mut self, gpio: &mut GPIO_PORT, color: Color) {
        match color {
            Color::Red => gpio.set[1].write(|w| w.setp22().set_bit()),
            Color::Yellow => gpio.set[0].write(|w| w.setp17().set_bit()),
            Color::Green => gpio.set[1].write(|w| w.setp16().set_bit()),
        };
    }

    /// Disable the specified LED.
    pub fn off(&mut self, gpio: &mut GPIO_PORT, color: Color) {
        match color {
            Color::Red => gpio.clr[1].write(|w| w.clrp022().set_bit()),
            Color::Yellow => gpio.clr[0].write(|w| w.clrp017().set_bit()),
            Color::Green => gpio.clr[1].write(|w| w.clrp016().set_bit()),
        };
    }

    /// Turn on all LEDs.
    pub fn all(&mut self, gpio: &mut GPIO_PORT) {
        self.on(gpio, Color::Red);
        self.on(gpio, Color::Yellow);
        self.on(gpio, Color::Green);
    }

    /// Turn off all LEDs.
    pub fn none(&mut self, gpio: &mut GPIO_PORT) {
        self.off(gpio, Color::Red);
        self.off(gpio, Color::Yellow);
        self.off(gpio, Color::Green);
    }
}
