use bitfield::{bitfield, Bit, BitRange};

#[derive(Copy, Clone, Default)]
pub struct U12(u16);

impl U12 {
    pub fn new(value: u16) -> Self {
        Self(value.min(0xFFF))
    }

    /// Return the inner u16 (with the 4 uppermost bits set to 0).
    pub fn as_u16(&self) -> u16 {
        self.0
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
    fn encode(&self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>, bit_index: &mut usize);
}

impl MeasurementValue for U12 {
    const SIZE: usize = 12;
    fn encode(&self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>, bit_index: &mut usize) {
        output.set_bit_range(*bit_index + Self::SIZE - 1, *bit_index, self.0);
        *bit_index += Self::SIZE;
    }
}

impl MeasurementValue for u16 {
    const SIZE: usize = 16;
    fn encode(&self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>, bit_index: &mut usize) {
        output.set_bit_range(*bit_index + Self::SIZE - 1, *bit_index, *self);
        *bit_index += Self::SIZE;
    }
}

bitfield! {
    pub struct EncodedMeasurement(MSB0 [u8]);
}

/// The encoder encodes `MeasurementValue`s into an `EncodedMeasurement` output buffer.
///
/// It keeps track of the offset and calculates the number of bytes written when finishing.
struct Encoder<'a> {
    bit_index: usize,
    data_mask: u8,
    output: &'a mut EncodedMeasurement<[u8; MAX_MSG_LEN]>,
}

impl<'a> Encoder<'a> {
    fn new(output: &'a mut EncodedMeasurement<[u8; MAX_MSG_LEN]>) -> Self {
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
        self.output.0[0] = self.data_mask;
        (self.bit_index + 4) / 8
    }
}

impl MeasurementMessage {
    /// Encode the measurement into the given buffer.
    ///
    /// Returns the number of bytes which should be sent
    pub fn encode(&self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>) -> usize {
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

        let mut output = EncodedMeasurement([0u8; MAX_MSG_LEN]);
        let length = input.encode(&mut output) as usize;

        assert_eq!(length, 1);
        assert_eq!(output.0[0..length], expeced_result);
    }

    #[test]
    fn test_measurement_encode_t_water() {
        let input = MeasurementMessage {
            t_water: Some(U12(0b0000_0101_1010)),
            ..MeasurementMessage::default()
        };
        let expeced_result = [1, 0b0000_0101, 0b1010_0000];
        let mut output = EncodedMeasurement([0u8; MAX_MSG_LEN]);

        let length = input.encode(&mut output) as usize;
        println!("{:012b}", input.t_water.unwrap().0);
        for b in &output.0[1..length] {
            print!("{:08b} ", b);
        }
        println!();
        assert_eq!(length, 3);
        assert_eq!(output.0[0..length], expeced_result);
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
        let mut output = EncodedMeasurement([0u8; MAX_MSG_LEN]);

        let length = input.encode(&mut output) as usize;
        println!("{:012b}", input.t_water.unwrap().0);
        for b in &output.0[1..length] {
            print!("{:08b} ", b);
        }
        println!();
        assert_eq!(length, MAX_MSG_LEN);
        assert_eq!(output.0[0..length], expeced_result);
    }
}
