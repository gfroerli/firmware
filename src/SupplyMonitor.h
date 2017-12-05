#include "mbed.h"

class SupplyMonitor {
public:
    SupplyMonitor(AnalogIn& input, DigitalOut& enable):
        _input(input),
        _enable(enable)
    {
        disable();
    }

    float read_input()
    { return _input.read(); }

    void enable()
    { _enable = 1; }
    
    void disable()
    { _enable = 0; }

    float get_supply_voltage();

private:
    AnalogIn& _input;
    DigitalOut& _enable;
};

