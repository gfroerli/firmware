#include <cstdint>
#include "mbed.h"

class OneWire {
public:
    OneWire(PinName pin):
        _pin(pin)
    {}

    void reset();

    void write_byte(uint8_t byte);
    void write_bit(bool bit);
    uint8_t read_byte();
    bool read_bit();

private:
    DigitalInOut _pin;
};
