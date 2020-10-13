use bitfield::{bitfield, Bit, BitRange};

#[derive(Copy, Clone, Default)]
pub struct U12(u16);

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
    fn encode(self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>, bit_index: &mut usize);
}

impl MeasurementValue for U12 {
    const SIZE: usize = 12;
    fn encode(self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>, bit_index: &mut usize) {
        output.set_bit_range(*bit_index + Self::SIZE - 1, *bit_index, self.0);
        *bit_index += Self::SIZE;
    }
}

impl MeasurementValue for u16 {
    const SIZE: usize = 16;
    fn encode(self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>, bit_index: &mut usize) {
        output.set_bit_range(*bit_index + Self::SIZE - 1, *bit_index, self);
        *bit_index += Self::SIZE;
    }
}

bitfield! {
    pub struct EncodedMeasurement(MSB0 [u8]);
}

impl MeasurementMessage {
    /// Encode the measurement into the given buffer.
    ///
    /// Returns the number of bytes which should be sent
    pub fn encode(&self, output: &mut EncodedMeasurement<[u8; MAX_MSG_LEN]>) -> usize {
        let mut data_mask = 0u8;
        let mut bit_index = 8;
        if let Some(t_water) = self.t_water {
            data_mask.set_bit(0, true);
            t_water.encode(output, &mut bit_index);
        }
        if let Some(t_inside) = self.t_inside {
            data_mask.set_bit(1, true);
            t_inside.encode(output, &mut bit_index);
        }
        if let Some(rh_inside) = self.rh_inside {
            data_mask.set_bit(2, true);
            rh_inside.encode(output, &mut bit_index);
        }
        if let Some(v_supply) = self.v_supply {
            data_mask.set_bit(3, true);
            v_supply.encode(output, &mut bit_index);
        }

        output.0[0] = data_mask;
        (bit_index + 4) / 8
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
