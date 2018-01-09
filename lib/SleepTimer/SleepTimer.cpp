#include "SleepTimer.h"

void SleepTimer::wait_ms(int millis)
{
    _wakup.set_ms(millis);
    //__WFI();
    //sleep();
}
