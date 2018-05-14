#include <cstdint>
#include "mbed.h"

class OneWire {
public:
    OneWire(PinName pin):
        _pin(pin),
        ticker(get_us_ticker_data())
    {}

    /**
     * Send a reset.
     * @return true if a slave sent a "presence" pulse
     */
    bool reset();

    void write_byte(uint8_t byte);
    void write_bit(bool bit);
    uint8_t read_byte();
    bool read_bit();

private:
    DigitalInOut _pin;
    // we access the ticker directly to avoid calling overhead
    const ticker_data_t *ticker;
};
