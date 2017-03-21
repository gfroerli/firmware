#include "DS18B20.h"


void DS18B20::send_command(Command command)
{
    _one_wire.reset();
    _one_wire.write_byte(static_cast<uint8_t>(Command::SkipROM));
    _one_wire.write_byte(static_cast<uint8_t>(command));
}

bool DS18B20::wait_for_completion()
{
    for(int n=0;n<100; ++n) {
        uint8_t byte = _one_wire.read_byte();
        if(byte) {
            return false;
        }
        wait_ms(10);
    }
    return true;
}

float DS18B20::read_temperature()
{
    send_command(Command::ReadScratchpad);
    int16_t temperature = _one_wire.read_byte();
    temperature |= _one_wire.read_byte() << 8;
    return static_cast<float>(temperature) * 0.0625f;
}
