#include "SupplyMonitor.h"
    
float SupplyMonitor::get_supply_voltage()
{
    enable();
    float input = read_input();
    disable();
    return convert_input(input);
}
