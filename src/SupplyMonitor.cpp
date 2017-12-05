#include "SupplyMonitor.h"
    
float SupplyMonitor::get_supply_voltage()
{
    enable();
    float input = read_input();
    disable();
    return input * 3.3f / 9.31 * (9.31f + 6.04f);
}
