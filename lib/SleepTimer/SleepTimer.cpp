#include "SleepTimer.h"

void SleepTimer::wait_ms(uint32_t millis)
{
    _wakeup.set_ms(millis);
    __WFI();
    //sleep();
}