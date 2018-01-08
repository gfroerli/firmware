#include "mbed.h"
#include "WakeUp.h"

class SleepTimer {
public:
    explicit SleepTimer(WakeUp& wakup):
        _wakup(wakup)
    { }

    void wait_ms(int millis);

private:
    WakeUp& _wakup;
};
