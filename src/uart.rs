
use lpc11uxx::{IOCON, USART, SYSCON, GPIO_PORT};

pub struct Uart;

impl Uart {
    /// Initialize the uart.
    ///
    /// * Set pin functions (USART, pull up)
    /// * Set pin direction (output)
    pub fn init(syscon: &mut SYSCON, iocon: &mut IOCON, usart: &mut USART, gpio: &mut GPIO_PORT) -> Self {

        // enable usart peripheral
        syscon.sysahbclkctrl.modify(|_,w| w.usart().enabled());

        unsafe {
            usart.fcr.fcr.write(|w| w
                                .fifoen().enabled()
                                .txfifores().clear_bit()
                                .rxfifores().clear_bit()
                                .rxtl().bits(0)
                                );
            usart.dlm.ier.write(|w| w.bits(0));
        }


        unsafe {
            // set baudrate. TODO calculate it ;)
            syscon.uartclkdiv.write(|w| w.bits(0x1));

            // set LCR[DLAB] to enable writing to divider registers
            usart.lcr.modify(|r,w| w
                             .bits(r.bits())
                             .dlab().enable_access_to_div());
            
            usart.dlm.dlm.write(|w| w.bits(0));
            usart.dll.dll.write(|w| w.bits(27));
            usart.fdr.write(|w| w.bits( 13 | (14 << 4)));

            usart.lcr.modify(|r,w| w
                             .bits(r.bits())
                             .dlab().disable_access_to_di());

            usart.lcr.write(|w| w
                            .wls()._8_bit_character_leng()
                            .sbs()._1_stop_bit()
                            .pe().disabled()
                            .bc().disable_break_transm());
        }

        // Set port functions
        (*iocon).pio1_13.write(|w| w
            .func().txd()
            .mode().pull_up());

        //(*iocon).pio1_13.write(|w| w
        //    .func().pio1_13()
        //    .mode().pull_up());

        //(*iocon).pio1_14.write(|w| w
        //    .func().rxd()
        //    .mode().pull_up());

        (*iocon).pio0_19.write(|w| w
            .func().txd()
            .mode().pull_up());

        //(*iocon).pio0_19.write(|w| w
        //    .func().pio0_19()
        //    .mode().floating());

        (*iocon).pio0_18.write(|w| w
            .func().rxd()
            .mode().pull_up());

        unsafe {
            // Set pin directions to output
            (*gpio).dir[1].modify(|r, w| w.bits(r.bits() | (1<<13)));

            // Set pin directions to output
            (*gpio).dir[0].modify(|r, w| w.bits(r.bits() | (1<<19)));
        }

        //gpio.clr[0].write(|w| w.clrp019().set_bit());
        //gpio.clr[1].write(|w| w.clrp013().set_bit());

        Uart { }
    }

    fn writable(&mut self, usart: &mut USART) -> bool {
        (usart.lsr.read().bits() & 0x20) != 0
    }

    pub fn putc(&mut self, usart: &mut USART, gpio: &mut GPIO_PORT, value: u8) {
        unsafe {
            /*
            if value > 0 {
                gpio.set[0].write(|w| w.setp19().set_bit());
            } else {
                gpio.clr[0].write(|w| w.clrp019().set_bit());
            }
            */
            while !self.writable(usart) {
            }
            usart.dll.thr.write(|w| w.bits(value.into()));
        }
    }
}
