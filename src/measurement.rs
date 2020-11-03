use bitfield::{Bit};
use bitvec::prelude::*;

#[derive(Copy, Clone, Default)]
pub struct U12(u16);

impl U12 {
    pub fn new(value: u16) -> U12 {
        U12(value.min(0xFFF))
    }
}

pub const MAX_MSG_LEN: usize = 8;

#[derive(Copy, Clone, Default)]
pub struct MeasurementMessage {
    pub t_water: Option<U12>,
    pub t_inside: Option<u16>,
    pub rh_inside: Option<u16>,
    pub v_supply: Option<U12>,
}

trait MeasurementValue {
    const SIZE: usize;
    fn encode(&self, output: &mut EncodedMeasurement, bit_index: &mut usize);
}

impl MeasurementValue for U12 {
    const SIZE: usize = 12;
    fn encode(&self, output: &mut EncodedMeasurement, bit_index: &mut usize) {
        output.0[*bit_index..*bit_index + Self::SIZE].store_be(self.0);
        *bit_index += Self::SIZE;
    }
}

impl MeasurementValue for u16 {
    const SIZE: usize = 16;
    fn encode(&self, output: &mut EncodedMeasurement, bit_index: &mut usize) {
        output.0[*bit_index..*bit_index + Self::SIZE].store_be(*self);
        *bit_index += Self::SIZE;
    }
}

// need to use `8` here, MAX_MSG_LEN doesn't compile
pub struct EncodedMeasurement(bitarr!(for 64, in Msb0, u8));

impl EncodedMeasurement {
    pub fn new() -> Self {
        EncodedMeasurement(bitarr![Msb0, u8; 0; MAX_MSG_LEN*8])
    }
}

/// The encoder encodes `MeasurementValue`s into an `EncodedMeasurement` output buffer.
///
/// It keeps track of the offset and calculates the number of bytes written when finishing.
struct Encoder<'a> {
    bit_index: usize,
    data_mask: u8,
    output: &'a mut EncodedMeasurement,
}

impl<'a> Encoder<'a> {
    fn new(output: &'a mut EncodedMeasurement) -> Self {
        Self {
            bit_index: 8,
            data_mask: 0,
            output,
        }
    }

    fn encode(&mut self, mask_bit: usize, value: &impl MeasurementValue) {
        value.encode(self.output, &mut self.bit_index);
        self.data_mask.set_bit(mask_bit, true);
    }

    /// Finish encoding, return the number of bytes encoded.
    fn finish(self) -> usize {
        self.output.0.as_mut_slice()[0] = self.data_mask;
        (self.bit_index + 4) / 8
    }
}

impl MeasurementMessage {
    /// Encode the measurement into the given buffer.
    ///
    /// Returns the number of bytes which should be sent
    pub fn encode(&self, output: &mut EncodedMeasurement) -> usize {
        let mut encoder = Encoder::new(output);
        if let Some(t_water) = self.t_water {
            encoder.encode(0, &t_water);
        }
        if let Some(t_inside) = self.t_inside {
            encoder.encode(1, &t_inside);
        }
        if let Some(rh_inside) = self.rh_inside {
            encoder.encode(2, &rh_inside);
        }
        if let Some(v_supply) = self.v_supply {
            encoder.encode(3, &v_supply);
        }
        encoder.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_encode_empty() {
        let input = MeasurementMessage::default();
        let expeced_result = [0];

        let mut output = EncodedMeasurement::new();
        let length = input.encode(&mut output) as usize;

        assert_eq!(length, 1);
        assert_eq!(output.0.as_slice()[0..length], expeced_result);
    }

    #[test]
    fn test_measurement_encode_t_water() {
        let input = MeasurementMessage {
            t_water: Some(U12(0b0000_0101_1010)),
            ..MeasurementMessage::default()
        };
        let expeced_result = [1, 0b0000_0101, 0b1010_0000];
        let mut output = EncodedMeasurement::new();

        let length = input.encode(&mut output) as usize;
        println!("{:012b}", input.t_water.unwrap().0);
        for b in &output.0.as_slice()[1..length] {
            print!("{:08b} ", b);
        }
        println!();
        assert_eq!(length, 3);
        assert_eq!(output.0.as_slice()[0..length], expeced_result);
    }

    #[test]
    fn test_measurement_encode_all() {
        let input = MeasurementMessage {
            t_water: Some(U12(0b0000_0101_1010)),
            t_inside: Some(0b1100_0011_1010_0101),
            rh_inside: Some(0b0011_1100_0101_1010),
            v_supply: Some(U12(0b1111_1010_0101)),
        };
        let expeced_result = [
            0x0F,
            0b0000_0101,
            0b1010_1100,
            0b0011_1010,
            0b0101_0011,
            0b1100_0101,
            0b1010_1111,
            0b1010_0101,
        ];
        let mut output = EncodedMeasurement::new();

        let length = input.encode(&mut output) as usize;
        println!("{:012b}", input.t_water.unwrap().0);
        for b in &output.0.as_slice()[1..length] {
            print!("{:08b} ", b);
        }
        println!();
        assert_eq!(length, MAX_MSG_LEN);
        assert_eq!(output.0.as_slice()[0..length], expeced_result);
    }
}
