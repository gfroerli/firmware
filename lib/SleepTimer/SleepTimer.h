#include "mbed.h"

class SleepTimer {
public:
    explicit SleepTimer(Timer& timer):
        _timer(timer)
    { }

    void wait_ms(int millis);

private:
    Timer& _timer;
};
