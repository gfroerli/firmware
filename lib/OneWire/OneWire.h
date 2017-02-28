#include <cstdint>
#include "mbed.h"

class OneWire {
public:
    OneWire(PinName pin):
        _pin(pin)
    {}

    void reset();

    void write_byte(uint8_t byte);
    uint8_t read_byte();
private:
    DigitalInOut _pin;
};
