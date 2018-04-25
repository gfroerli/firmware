#include "mbed.h"
#include <functional>
#include "DS18B20.h"
#include "RN2483.h"
#include "secrets.h"
#include "SupplyMonitor.h"
#include "PinMapping.h"

// SHT configuration
const uint8_t SHT2X_I2C_ADDR = 0x40<<1;

// LoRaWAN settings
const bool USE_ADR = true;

// Measurement interval
const uint32_t INTERVAL_S = 30;

// Registers
static uint32_t* PIO0_19 = (uint32_t*)0x4004404C;
static const uint32_t PIO0_19_RESET_VALUE = 0x00000090;
static const uint32_t PIO0_19_UART_VALUE = 0x00000091;

// UART to use for debug messages
Serial uart1(UART1_TX, NC, 57600);


// Helper functions

float calculate_temp(char msb, char lsb) {
    lsb &= 0xFC;
    return -46.85 + 175.72 * (msb<<8 | lsb) / 65536.0f;
}

float calculate_humi(char msb, char lsb) {
    lsb &= 0xFC;
    return -6 + 125.0f * (msb<<8 | lsb) / 65536.0f;
}

int send_command(I2C& i2c, uint8_t address, uint8_t command) {
    return i2c.write(address, (char*)&command, 1);
}

/**
 * Switch the UART to RN2483 mode.
 */
inline void uart_rn(uint32_t prewait_ms, uint32_t postwait_ms) {
    wait_ms(prewait_ms);
    *PIO0_19 = PIO0_19_UART_VALUE;
    wait_ms(postwait_ms);
}

/**
 * Switch the UART to logging mode.
 */
inline void uart_log(uint32_t prewait_ms, uint32_t postwait_ms) {
    wait_ms(prewait_ms);
    *PIO0_19 = PIO0_19_RESET_VALUE;
    wait_ms(postwait_ms);
}

/**
 * Interrupt handler for watchdog timer.
 */
void wakeup_irq_handler() {
    uart1.printf("wakeup_irq_handler called\n");

    // The Watchdog interrupt flag (WDINT) is set when the Watchdog counter
    // reaches the value specified by WARNINT. This flag is cleared when any
    // reset occurs, and is cleared by software by writing a 1 to this bit.
    LPC_WWDT->MOD = (1<<3);
}

/**
 * Power down the MCU and sleep for the specified number of milliseconds.
 *
 * This is based on datasheet sections 3.9.5.2 ("Programming Power-down mode")
 * and 3.9.5.3 (Wake-up from Power-down mode).
 */
void power_down(uint32_t duration_ms) {
    uart_log(0, 1);
    uart1.printf("Powering down for %i ms...\n", duration_ms);
    wait_ms(1);

    // 1. The PD bits in the Power control register (PCON) must be set to 0x2
    LPC_PMU->PCON = 0x2; // ARM WFI will enter Power-down mode

    // 2. Select the power configuration in power-down mode in the deep-sleep
    // configuration register (PDSLEEPCFG)
    LPC_SYSCON->PDSLEEPCFG &= ~(1<<6); // Enable watchdog in power-down mode

    // 3. Select the watchdog oscillator as the WWDT clock source in the watchdog clock select register (CLKSEL)
    LPC_SYSCON->SYSAHBCLKCTRL |= (1<<15); // Enable clock for WWDT
    LPC_SYSCON->PDRUNCFG &= ~(1<<6); // Enable Watchdog oscillator
    LPC_SYSCON->WDTOSCCTRL = 14 | (1<<5); // Set oscillator for 20 kHz = 5 kHz after divide by 4 in WDT
    LPC_WWDT->CLKSEL = 1; // Select watchdog oscillator as clock source
    LPC_WWDT->MOD = 0x1; // Enable watchdog timer

    // Configure watchdog timer
    float cycles_per_ms = 5.0; // This *might* be fine without calibration, TODO test it!
    LPC_WWDT->TC = (uint32_t)((float)duration_ms * cycles_per_ms);
    LPC_WWDT->WARNINT = 0;

    // 4. If the main clock is not the IRC, power up the IRC in the power
    // configuration register (PDRUNCFG) and switch the clock source to IRC in
    // the main clock source select register (MAINCLKSEL). This ensures that
    // the system clock is shut down glitch-free.
    LPC_SYSCON->PDRUNCFG &= ~(1<<1); // Enable IRC oscillator
    LPC_SYSCON->MAINCLKSEL = 0x0; // Clock source for main clock: IRC oscillator

    // 5. Select the power configuration after wake-up in the wake-up
    // configuration register (PDAWAKECFG)
    // LPC_SYSCON->PDAWAKECFG = ; TODO
    LPC_SYSCON->PDAWAKECFG = 0xE800 // reserved bits
                           | (1<<10) // usb transceiver powered down
                           | (1<<8) // usb pll powered down
                           ; // TODO: Maybe we can power down even more parts?

    // 6. If any of the available wake-up interrupts are used for wake-up,
    // enable the interrupts in the interrupt wake-up registers and in the NVIC.
    LPC_SYSCON->STARTERP0 = 0; // Disable all pin interrupts
    LPC_SYSCON->STARTERP1 |= (1<<12); // Enable WWDT interrupt
    LPC_SYSCON->STARTERP1 &= ~(1<<13); // Disable BOD interrupt
    LPC_SYSCON->STARTERP1 &= ~(1<<19); // Disable USB interrupt
    NVIC_SetVector(WDT_IRQn, (uint32_t)wakeup_irq_handler);

    // 7. Write one to the SLEEPDEEP bit in the ARM Cortex-M0 SCR register.
    SCB->SCR |= SCB_SCR_SLEEPDEEP_Msk;

    // Start wakeup timer
    LPC_WWDT->FEED = 0xAA; LPC_WWDT->FEED = 0x55;
    NVIC_EnableIRQ(WDT_IRQn);

    // 8. Use the ARM WFI instruction.
    __WFI();

    uart1.printf("Woke up from power-down! Back to work.\n");
}


int main() {
    uart_log(0, 0); // Make sure that we set up the UART port for logging

    uart1.baud(57600);
    uart1.printf("Start the super awesome water temperature sensor reader\n");

    wait_ms(5);

    // Initialize LEDs
    DigitalOut led_red(LED_RED);
    DigitalOut led_yellow(LED_YELLOW);
    DigitalOut led_green(LED_GREEN);
    uart1.printf("LEDs initialized\n");

    // Initialize supply monitor
    DigitalOut supply_monitor_enable(SUPPLY_MONITOR_ENABLE);
    AnalogIn supply_monitor_input(SUPPLY_MONITOR_INPUT);
    SupplyMonitor supply_monitor(supply_monitor_input, supply_monitor_enable);
    uart1.printf("SupplyMonitor initialized\n");

    // Initialize DS18B20 sensor
    OneWire one_wire(DS18B20_IO);
    DS18B20 ds18b20(one_wire);
    uart1.printf("DS18B20 initialized\n");

    // Initialize SHT21 sensor
    I2C i2c_1(SDA, SCL);
    i2c_1.frequency(20000);
    uart1.printf("I2C initialized\n");

    // Initialize the RN2483 module
    RN2483 lora(RN2483_TX, RN2483_RX);

    uart1.printf("RN2483 initialized\n");

    led_red = 0;
    led_yellow = 0;
    led_green = 0;

    do {
        led_yellow = 1;
        uint8_t buffer[17] = {};

        uart_rn(0, 0);
        uint8_t bytes = lora.getHWEUI(buffer, 17);
        uart_log(0, 1);

        if (bytes) {
            led_green = 1;
            led_red = 0;
            uart1.printf("HWEUI: ");
            for (uint8_t n = 0; n < bytes; ++n) {
                uart1.printf("%02x", buffer[n]);
            }
            uart1.printf("\n");
        } else {
            led_green = 0;
            led_red = 1;
        }
        led_yellow = 0;
        wait(1.0);
    } while (DEV_EUI[0] == 0 && APP_EUI[0] == 0 && APP_KEY[0] == 0);

    // Join the network
    uart_rn(1, 0);
    bool joined = lora.isJoined();
    uart_log(0, 1);
    while (!joined) {
        led_yellow = 1;
        uart1.printf("Joining TTN via OTAA...\n");

        uart_rn(1, 0);
        joined = lora.initOTAA(DEV_EUI, APP_EUI, APP_KEY, USE_ADR);
        uart_log(1, 1);
        if (joined) {
            led_green = 1;
            led_red = 0;
            uart1.printf("Joined TTN successfully!\n");
        } else {
            led_red = 1;
            uart1.printf("Joining TTN failed\n");
        }
        led_yellow = 0;

        wait(5.0);
    }

    // Main loop
    while(true) {
        uart1.printf("------\nStart measurement...\n");

        led_green = 1;
        wait(0.2);

        int error;

        // Start conversion
        ds18b20.start_measurement();

        // Start temperature measurement without clock stretching
        error = send_command(i2c_1, SHT2X_I2C_ADDR, (uint8_t)0xF3);
        if (error) {
            uart1.printf("i2c.write failed: %i\n", error);
        }
        wait(0.1);

        static const size_t len = 3;
        char data[len] = {};
        error = i2c_1.read(SHT2X_I2C_ADDR, data, len);
        if (error) {
            uart1.printf("i2c_1.read failed: %i\n", error);
        }
        for(size_t i=0; i<len; ++i) {
            uart1.printf("%02x", data[i]);
        }
        float sht_temp = calculate_temp(data[0], data[1]);
        uart1.printf(" -> SHT21 Temp=%u\n", (unsigned)(sht_temp*1000.0f));

        // Start humidity measurement without clock stretching
        error = send_command(i2c_1, SHT2X_I2C_ADDR, (uint8_t)0xF5);
        if (error) {
            uart1.printf("i2c.write failed: %i\n", error);
        }
        wait(0.1);
        error = i2c_1.read(SHT2X_I2C_ADDR, data, len);
        if (error) {
            uart1.printf("i2c_1.read failed: %i\n", error);
        }
        for(size_t i=0; i<len; ++i) {
            uart1.printf("%02x", data[i]);
        }
        float sht_humi = calculate_humi(data[0], data[1]);
        uart1.printf(" -> SHT21 Humi=%u\n", (unsigned)(sht_humi*1000.0f));

        bool timeout = ds18b20.wait_for_completion();
        if (timeout) {
            uart1.printf("Conversion timed out");
        }

        float ds_temp = ds18b20.read_temperature();
        uart1.printf("1-Wire Temp: %u\n", (unsigned)(ds_temp*1000.0f));

        float supply_voltage = supply_monitor.get_supply_voltage();

        led_green = 0;
        wait(0.2);

        // Measurements done, prepare payload
        led_yellow = 1;
        uint8_t payload[16] = {};
        memcpy(payload, &ds_temp, 4);
        memcpy(payload + 4, &sht_temp, 4);
        memcpy(payload + 8, &sht_humi, 4);
        memcpy(payload + 12, &supply_voltage, 4);

        // UART: Talk to RN2483
        uart_rn(0, 0);

        // Wake up RN2483
        lora.wakeUp();
        wait_ms(10);

        // Send payload to TTN via LoRaWAN
        lora.send(1, payload, 16);
        led_yellow = 0;

        // Put RN2483 in sleep mode
        wait_ms(10);
        lora.sleep();

        // UART: Back to logging mode
        wait_ms(10);
        uart_log(1, 0);
        wait_ms(10);

        //power_down(INTERVAL_S * 1000);
        power_down(5000);
    }
}
