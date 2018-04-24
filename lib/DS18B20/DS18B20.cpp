#include "DS18B20.h"
#include <iterator>
#include <limits>
#include <array>

DS18B20::Result DS18B20::send_command(Command command)
{
    bool present = _one_wire.reset();
    if (!present) {
        return Result::NoDevice;
    }
    _one_wire.write_byte(static_cast<uint8_t>(Command::SkipROM));
    _one_wire.write_byte(static_cast<uint8_t>(command));
    return Result::Success;
}

bool DS18B20::wait_for_completion()
{
    for (int n = 0; n < 100; ++n) {
        if (_one_wire.read_byte()) {
            return false;
        }
        wait_ms(10);
    }
    return true;
}

uint8_t DS18B20::crc8(const uint8_t* begin, const uint8_t* end){
    uint8_t crc = 0;
    for (const uint8_t* it = begin; it != end; ++it) {
        uint8_t inbyte = *it;
        for (int i = 0; i < 8; ++i) {
            uint8_t mix = (crc ^ inbyte) & 0x01;
            crc >>= 1;
            if (mix) {
                crc ^= 0x8C;
            }
            inbyte >>= 1;
        }
    }
    return crc;
}

float DS18B20::read_temperature()
{
    send_command(Command::ReadScratchpad);

    std::array<uint8_t, 9> data = {};
    for (auto& byte: data) {
        byte = _one_wire.read_byte();
    }

    uint8_t crc = crc8(data.begin(), data.end()-1);

    if (data.back() != crc) {
        return std::numeric_limits<float>::quiet_NaN();
    }

    int16_t temperature = data[0] + (data[1] << 8);

    return static_cast<float>(temperature) * 0.0625f;
}
