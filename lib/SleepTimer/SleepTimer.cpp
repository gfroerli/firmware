#include "SleepTimer.h"

void SleepTimer::wait_ms(int millis)
{
    auto start = _timer.read_ms();
    while (_timer.read_ms() < (start + millis)) {
        sleep();
    }
}
