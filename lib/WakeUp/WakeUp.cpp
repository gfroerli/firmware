/**
Due to lack of another option for the LPC11u24 the watchdog timer is used as wakeup source.
Since if the reset on watchdog event bit is set, I cannot remove it again, this means if you also got watchdog code running
the most likely result is that it just resets your board.
**/


#ifdef TARGET_LPC11U24

#include "WakeUp.h"

Callback<void()> WakeUp::callback;
float WakeUp::cycles_per_ms = 5.0;

void WakeUp::set_ms(uint32_t ms)
{
    if (ms != 0) {
        LPC_SYSCON->SYSAHBCLKCTRL |= 0x8000;
        LPC_SYSCON->PDRUNCFG &= ~(1<<6);
        LPC_SYSCON->PDSLEEPCFG &= ~(1<<6);
        LPC_SYSCON->STARTERP1 |= 1<<12;
        
        //Set oscillator for 20kHz = 5kHz after divide by 4 in WDT
        LPC_SYSCON->WDTOSCCTRL = 14 | (1<<5);
        
        LPC_WWDT->MOD = 1;      //Enable WDT
        LPC_WWDT->TC = (uint32_t)((float)ms * cycles_per_ms);
        LPC_WWDT->CLKSEL = 1;   //WDTOSC
        LPC_WWDT->WARNINT = 0;
        
        NVIC_SetVector(WDT_IRQn, (uint32_t)WakeUp::irq_handler);
        
        //Feeeeeed me
        LPC_WWDT->FEED = 0xAA;
        LPC_WWDT->FEED = 0x55;
        
        NVIC_EnableIRQ(WDT_IRQn);
    } else
        NVIC_DisableIRQ(WDT_IRQn);
    
}

void WakeUp::irq_handler(void)
{
    LPC_WWDT->MOD = 1<<3;
    callback.call();
}

void WakeUp::calibrate(void)
{
    cycles_per_ms = 5.0;
    set_ms(1100);
    wait_ms(10);    //Give time to sync
    uint32_t count1 = LPC_WWDT->TV;
    wait_ms(100);
    uint32_t count2 = LPC_WWDT->TV;
    set_ms(0);
    count1 = count1 - count2;
    
    cycles_per_ms = count1 / 100.0;
}

#endif
