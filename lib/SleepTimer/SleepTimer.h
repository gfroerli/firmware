#include "mbed.h"
#include "WakeUp.h"

class SleepTimer {
public:
    explicit SleepTimer(WakeUp& wakup):
        _wakup(wakup)
    { }

    void wait_ms(uint32_t millis);

private:
    WakeUp& _wakup;
};
