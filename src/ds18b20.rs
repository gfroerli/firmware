use embedded_hal::{
    blocking::delay::{DelayMs, DelayUs},
    digital::v2::{InputPin, OutputPin},
};
use one_wire_bus::{Address, OneWire, OneWireError, OneWireResult};

/// Family code of the DS18B20
const FAMILY_CODE_DS18B20: u8 = 0x28;

/// Command bytes that can be sent to the DS18B20
mod commands {
    /// Convert temperature
    ///
    /// This command initiates a single temperature conversion. Following the conversion, the
    /// resulting thermal data is stored in the 2-byte temperature register in the scratch-pad memory
    /// and the DS18B20 returns to its low-power idle state.
    pub const CONVERT_TEMP: u8 = 0x44;

    /// Read scratchpad
    ///
    /// This command allows the master to read the contents of the scratchpad. The data transfer
    /// starts with the least sig-nificant bit of byte 0 and continues through the scratchpad until
    /// the 9th byte (byte 8 – CRC) is read. The master may issue a reset to terminate reading at
    /// any time if only part of the scratchpad data is needed.
    pub const READ_SCRATCHPAD: u8 = 0xBE;
}

pub struct Ds18b20(Address);

impl Ds18b20 {
    /// Scan the one-wire bus for a DS18B20 sensor. Return the first sensor found.
    pub fn find<P, E>(
        one_wire_bus: &mut OneWire<P>,
        delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    ) -> OneWireResult<Self, E>
    where
        P: OutputPin<Error = E> + InputPin<Error = E>,
    {
        for device_address in one_wire_bus.devices(false, delay) {
            let addr = device_address?;
            if addr.family_code() == FAMILY_CODE_DS18B20 {
                return Ok(Self(addr));
            }
        }
        Err(OneWireError::Timeout)
    }

    /// Start a temperature measurement.
    pub fn start_measurement<P, E>(
        &self,
        one_wire_bus: &mut OneWire<P>,
        delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    ) -> OneWireResult<(), E>
    where
        P: OutputPin<Error = E> + InputPin<Error = E>,
    {
        one_wire_bus.send_command(commands::CONVERT_TEMP, Some(&self.0), delay)
    }

    /// Return the raw DS18B20 temperature data from the scratchpad register.
    ///
    /// NOTE: The resolution of the temperature sensor is user-configurable to 9, 10, 11, or
    /// 12 bits, corresponding to increments of 0.5°C, 0.25°C, 0.125°C, and 0.0625°C, respectively.
    /// The default resolution at power-up is 12-bit. Because we never set the resolution, we
    /// can rely on the fact that it's always 12-bit (increments of 0.0625°C).
    pub fn read_raw_temperature_data<P, E>(
        &self,
        one_wire_bus: &mut OneWire<P>,
        delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    ) -> OneWireResult<u16, E>
    where
        P: OutputPin<Error = E> + InputPin<Error = E>,
    {
        one_wire_bus.send_command(commands::READ_SCRATCHPAD, Some(&self.0), delay)?;

        // We're only interested in the first two bytes, but we still want to read 9 bytes
        // in order to be able to verify the CRC.
        let mut scratchpad = [0; 9];
        one_wire_bus.read_bytes(&mut scratchpad, delay)?;
        one_wire_bus::crc::check_crc8(&scratchpad)?;

        // 12-bit raw temperature data is in bytes 0 and 1
        if cfg!(feature = "dev") {
            assert!(
                (scratchpad[1] & 0xf0) == 0,
                "Raw data contains more than 12 data bits"
            );
        }
        Ok(u16::from_le_bytes([scratchpad[0], scratchpad[1]]))
    }
}
