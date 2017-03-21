#include <cstdint>
#include "mbed.h"
#include "OneWire.h"

class DS18B20 {
public:
    enum class Command: uint8_t {
        SkipROM         = 0xCC,
        StartConversion = 0x44,
        ReadScratchpad  = 0xBE,
    };

    DS18B20(OneWire& one_wire):
        _one_wire(one_wire)
    {}

    void send_command(Command command);

    void start_measurement()
    { send_command(Command::StartConversion); }

    bool wait_for_completion();

    float read_temperature();

private:
    OneWire& _one_wire;
};

