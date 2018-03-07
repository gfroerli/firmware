/**
Due to lack of another option for the LPC11u24 the watchdog timer is used as wakeup source.
Since if the reset on watchdog event bit is set, I cannot remove it again, this means if you also got watchdog code running
the most likely result is that it just resets your board.
**/


#ifdef TARGET_LPC11U24

#include "WakeUp.h"

float WakeUp::cycles_per_ms = 5.0;

void WakeUp::set_ms(uint32_t ms)
{
    if (ms != 0) {
        LPC_PMU->PCON = 0x1;

        LPC_SYSCON->SYSAHBCLKCTRL |= (1<<15);
        LPC_SYSCON->PDRUNCFG &= ~(1<<1); // Enable IRC oscillator
        LPC_SYSCON->PDRUNCFG &= ~(1<<6); // Enable Watchdog oscillator
        LPC_SYSCON->PDSLEEPCFG &= ~(1<<6); // Enable watchdog in power-down mode

        //LPC_SYSCON->MAINCLKSEL = 0;
        //LPC_SYSCON->MAINCLKUEN = 0;
        //LPC_SYSCON->MAINCLKUEN = 1;

        LPC_SYSCON->PDAWAKECFG = 0xED00;

        LPC_SYSCON->STARTERP1 |= 1<<12; // Enable WWDT intterupt

        //Set oscillator for 20kHz = 5kHz after divide by 4 in WDT
        LPC_SYSCON->WDTOSCCTRL = 14 | (1<<5);
        
        LPC_WWDT->TC = 5 * 5000;
        LPC_WWDT->CLKSEL = 1;   //WDTOSC
        LPC_WWDT->WARNINT = 0;
        
        LPC_WWDT->MOD = 1;      //Enable WDT

        wait_ms(1);

        NVIC_SetVector(WDT_IRQn, (uint32_t)WakeUp::irq_handler);
        
        //Feeeeeed me
        LPC_WWDT->FEED = 0xAA;
        LPC_WWDT->FEED = 0x55;


        NVIC_ClearPendingIRQ(WDT_IRQn);
        //(*ISER0) |= (1 << 25);
        NVIC_EnableIRQ(WDT_IRQn);

        SCB->SCR |= SCB_SCR_SLEEPDEEP_Msk;
    } else {
        NVIC_DisableIRQ(WDT_IRQn);
    }
}

void WakeUp::irq_handler(void)
{
    LPC_WWDT->MOD = 1<<3;
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
