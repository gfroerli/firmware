#include "OneWire.h"

void OneWire::reset()
{
    DigitalOut temperature_pin(_pin);
    temperature_pin = 0;
    wait_us(500);
    DigitalIn temperature_pin_in(_pin);
    wait_us(500);
}

// output byte d (least sig bit first).
void OneWire::write_byte(unsigned char d)
{
    unsigned char n;

    for (n=8; n!=0; n--)
    {
        // test least sig bit
        if (d & 0x01)
        {
            DigitalOut temperature_pin(_pin);
            temperature_pin = 0;
            wait_us(5);
            DigitalIn temperature_pin_in(_pin);
            wait_us(60);
        }
        else
        {
            DigitalOut temperature_pin(_pin);
            temperature_pin = 0;
            wait_us(60);
            DigitalIn temperature_pin_in(_pin);
        }
        // now the next bit is in the least sig bit position.
        d = d >> 1;
    }

}

// read byte, least sig byte first
unsigned char OneWire::read_byte()
{
    uint8_t d = 0;

    for (uint8_t n = 0; n < 8; n++)
    {
        DigitalOut temperature_pin(_pin);
        temperature_pin = 0;
        wait_us(5);
        DigitalIn temperature_pin_in(_pin);
        wait_us(5);
        uint8_t b = temperature_pin_in;
        wait_us(50);
        // shift d to right and insert b in most sig bit position
        d = (d >> 1) | (b<<7);
    }
    return d;
}
