#include "OneWire.h"

/**
 * Reset 1-wire device and wait for presence pulse
 */
bool OneWire::reset()
{
    // The master starts a transmission with a reset pulse, which pulls the
    // wire to 0 volts for at least 480 µs. This resets every slave device on
    // the bus.
    _pin.write(0);
    _pin.output();
    wait_us(500);

    // After that, any slave device, if present, shows that it exists with a
    // "presence" pulse: it holds the bus low for at least 60 µs after the
    // master releases the bus.
    _pin.input();
    wait_us(50);
    bool b = _pin;

    // The total reset time must be at least 960 µs.
    wait_us(450);

    return !b;
}

void OneWire::write_bit(bool bit)
{
    us_timestamp_t start = ticker_read_us(ticker);
    _pin.write(0);
    _pin.output();
    if (bit) {
        while((ticker_read_us(ticker) - start) < 6u);
        _pin.input();
        wait_us(55);
    } else {
        while((ticker_read_us(ticker) - start) < 60u);
        _pin.input();
    }
    while((ticker_read_us(ticker) - start) < 70u);
}

// output byte d (least sig bit first).
void OneWire::write_byte(uint8_t d)
{
    for (uint8_t n = 0; n < 8; ++n)
    {
        write_bit(d & 0x01);
        d = d >> 1;
    }
}

bool OneWire::read_bit() {
    us_timestamp_t start = ticker_read_us(ticker);
    _pin.write(0);
    _pin.output();
    while((ticker_read_us(ticker) - start) < 6u);
    _pin.input();
    while((ticker_read_us(ticker) - start) < 15u);
    bool b = _pin.read();
    while((ticker_read_us(ticker) - start) < 70u);
    return b;
}

// read byte, least sig byte first
uint8_t OneWire::read_byte()
{
    uint8_t d = 0;

    for (uint8_t n = 0; n < 8; ++n)
    {
        if (read_bit()) {
            d |= 1 << n;
        }
    }
    return d;
}
