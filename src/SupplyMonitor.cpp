#include "SupplyMonitor.h"
    
float SupplyMonitor::get_supply_voltage()
{
    enable();
    float input = 0.0f;
    for (size_t n = 0; n < _samples; ++n) {
        input += read_input();
    }
    disable();
    input /= static_cast<float>(_samples);
    return input * 3.3f / 9.31 * (9.31f + 6.04f);
}
