
use lpc11uxx::{IOCON, GPIO_PORT};

pub struct Uart;

impl Uart {
    /// Initialize the LEDs.
    ///
    /// * Set pin functions (GPIO, no pull up/down)
    /// * Set pin direction (output)
    pub fn init(iocon: &mut IOCON, gpio: &mut GPIO_PORT) -> Self {
            // Set port functions
            (*iocon).pio1_13.write(|w| w
                .func().txd()
                .mode().inactive());
            (*iocon).pio1_14.write(|w| w
                .func().rxd()
                .mode().inactive());

        unsafe {
            // Set pin directions to output
            (*gpio).dir[1].modify(|r, w| w.bits(r.bits() | (1<<14)));
        }

        Uart { }
    }
}
