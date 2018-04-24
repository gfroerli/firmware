#include "mbed.h"
#include "WakeUp.h"

class SleepTimer {
public:
    explicit SleepTimer(WakeUp& wakeup):
        _wakeup(wakeup)
    { }

    void wait_ms(uint32_t millis);

private:
    WakeUp& _wakeup;
};
