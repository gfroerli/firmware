#include "power_down.h"
#include "mbed.h"

void disable_unused_peripherals()
{
    LPC_SYSCON->SYSAHBCLKCTRL &= ~
        (1 << 11) | // SSP0
        (1 << 14) | // USB
        (1 << 18) | // SSP1
        (1 << 19) | // PINT
        (1 << 23) | // GROUP0INT
        (1 << 24) | // GROUP1INT
        (1 << 27);  // USBRAM
}

void disable_used_peripherals()
{
    LPC_SYSCON->SYSAHBCLKCTRL &= ~
        (1 <<  5) | // I2C
        (1 <<  6) | // GPIO
        (1 << 12) | // USART
        (1 << 13) | // ADC
        (1 << 16);  // IOCON
}

void enable_used_peripherals()
{
    LPC_SYSCON->SYSAHBCLKCTRL |=
        (1 <<  5) | // I2C
        (1 <<  6) | // GPIO
        (1 << 12) | // USART
        (1 << 13) | // ADC
        (1 << 16);  // IOCON
}
