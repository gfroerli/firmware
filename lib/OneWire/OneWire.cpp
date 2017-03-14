#include "OneWire.h"

void OneWire::reset()
{
    _pin.output();
    _pin = 0;
    wait_us(500);
    _pin.input();
    wait_us(500);
}

void OneWire::write_bit(bool bit)
{
    if (bit) {
        _pin.output();
        _pin = 0;
        wait_us(5);
        _pin.input();
        wait_us(55);
    } else {
        _pin.output();
        _pin = 0;
        wait_us(60);
        _pin.input();
    }
}

// output byte d (least sig bit first).
void OneWire::write_byte(unsigned char d)
{
    for (uint8_t n=8; n!=0; n--)
    {
        write_bit(d & 0x01);
        d = d >> 1;
    }
}

bool OneWire::read_bit() {
    _pin.output();
    _pin = 0;
    wait_us(5);
    _pin.input();
    wait_us(5);
    bool b = _pin;
    wait_us(50);

    return b;
}

// read byte, least sig byte first
uint8_t OneWire::read_byte()
{
    uint8_t d = 0;

    for (uint8_t n = 0; n < 8; n++)
    {
        if (read_bit()) {
            d |= 1 << n;
        }
    }
    return d;
}
