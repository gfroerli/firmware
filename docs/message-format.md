# Message Format

We use the FPort value to determine the message format which is used for the
message.

## Legacy Format (FPort = 1)

The legacy format is just for little endian floats:

`[T_water, T_inside, RH_inside, V_supply]`

## New Format (FPort = 2)

The message consist of one header byte `data_mask` followed by a variable
amount of data:

```
|data_mask|data|
```

The `data_mask` has a bit set for every value that is available. This allows to
send up to 8 values:

|bit     |value    |type|conversion                           |unit|
|--------|---------|----|-------------------------------------|----|
|xxxxxxx1|T_water  |u12 |t / 16.0                             |°C  |
|xxxxxx1x|T_inside |u16 |-45 + 175 * (val / 2^16)             |°C  |
|xxxxx1xx|RH_inside|u16 |100 * (v / 2^16)                     |%RH |
|xxxx1xxx|V_supply |u12 |v / 4095 * 3.3 / 9.31 * (9.31 + 6.04)|V   |
|xxx1xxxx|reserved | -  |                                     |    |
|xx1xxxxx|reserved | -  |                                     |    |
|x1xxxxxx|reserved | -  |                                     |    |
|1xxxxxxx|reserved | -  |                                     |    |

The order of the values is the order in the table above.

Since message always consists of whole bytes we pad any remaining bits with
zeros.

## Examples

If we have just `T_water=0b0000_0101_1010` we get the following frame:

`|0000_0001|0000_0101|1010_0000|`.

Note the 4 padding zeros at the end.

If we have `T_water=0b0000_0101_1010` and `V_supply=0b1111_0001_1000` available
we'd get the following frame:

`|00001001|0000_0101|1010_1111|0001_1000|`

Note that to store the two 12 bit values we only need 3 payload bytes.


## Code

The code to implement the message format is found here:
[../firmware/src/measurement.rs](../firmware/src/measurement.rs)
