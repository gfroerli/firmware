# Message Format

The message consist of one header byte `data_mask` followed by a variable
amount of data:

```
|data_mask|data|
```

The `data_mask` has a bit set for every value that is available. This allows to
send up to 8 values:

|bit     |value    |type|
|--------|---------|----|
|xxxxxxx1|T_water  |u12 |
|xxxxxx1x|T_inside |u16 |
|xxxxx1xx|RH_inside|u16 |
|xxxx1xxx|V_supply |u12 |
|xxx1xxxx|reserved | -  |
|xx1xxxxx|reserved | -  |
|x1xxxxxx|reserved | -  |
|1xxxxxxx|reserved | -  |

## Examples

If we have just `T_water=0b0000_0101_1010` we get the following frame:

`|0000_0001|0000_0101|1010_0000|`.

If we just have `T_water=0b0000_0101_1010` and `V_supply=0b1111_0001_1000` available we'd have the following
frame:

`|00001001|0000_0101|1010_1111|0001_1000|`


## Code

The code to implement the message format is found here:
[../firmware/src/measurement.rs](../firmware/src/measurement.rs)
