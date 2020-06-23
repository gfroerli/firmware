use core::fmt::{Debug, Write};

use ds18b20::{self, Ds18b20};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use one_wire_bus::{OneWire, OneWireError, OneWireResult};

pub fn create_and_scan_one_wire<P, E>(
    delay: &mut impl DelayUs<u16>,
    tx: &mut impl Write,
    one_wire_pin: P,
) -> OneWire<P>
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    let mut one_wire_bus = OneWire::new(one_wire_pin).unwrap();
    for device_address in one_wire_bus.devices(false, delay) {
        // The search could fail at any time, so check each result. The iterator automatically
        // ends after an error.
        let device_address = device_address.unwrap();

        // The family code can be used to identify the type of device
        // If supported, another crate can be used to interact with that device at the given address
        writeln!(
            tx,
            "Found device at address {:?} with family code: {:#x?}",
            device_address,
            device_address.family_code()
        )
        .unwrap();
    }
    one_wire_bus
}

pub fn get_ds18b20_sensor<P, E>(
    delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    one_wire_bus: &mut OneWire<P>,
) -> OneWireResult<Ds18b20, E>
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
{
    for device_address in one_wire_bus.devices(false, delay) {
        let device_address = device_address?;
        if device_address.family_code() != ds18b20::FAMILY_CODE {
            // skip other devices
            continue;
        }
        // You will generally create the sensor once, and save it for later
        return Ds18b20::new(device_address);
    }
    Err(OneWireError::Timeout)
}
