# Supply Monitor

## Measurement

Measurement done at Coredump with `psu-rn` and `UT61E` multimeter.

VDDA was measured to be 3.28V at Vbat = 4.51V

|Vbat|VREF Raw|VDDA|Vin raw|Vin new|Vin old|
|----|--------|----|-------|-------|-------|
|5.00| 1580   |3.15| 3850  | 4.89  | 5.12  |
|4.51| 1580   |3.15| 3476  | 4.41  | 4.62  |
|4.01| 1580   |3.15| 3104  | 3.94  | 4.12  |
|3.50| 1580   |3.15| 2716  | 3.45  | 3.61  |
|3.30| 1580   |3.15| 2568  | 3.25  | 3.42  |
|3.00| 1736   |2.87| 2563  | 2.97  | 3.41  |
|2.60| 2006   |2.49| 2578  | 2.58  | 3.43  |

The new methods seems to calculate VDDA slightly wrong and thus calculates a
too low value for Vin. But the error seems to be similar than with the old
measurement method.
