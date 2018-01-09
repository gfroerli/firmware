#include "mbed.h"
#include <functional>
#include "DS18B20.h"
#include "RN2483.h"
#include "secrets.h"
#include "SupplyMonitor.h"
#include "PinMapping.h"
#include "SleepTimer.h"
#include "power_down.h" 

// SHT configuration
const uint8_t SHT2X_I2C_ADDR = 0x40<<1;

// LoRaWAN settings
const bool USE_ADR = true;

// Measurement interval
const float INTERVAL = 300.0;

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


// UART to use for debug messages
Serial uart1(UART1_TX, NC, 57600);

static uint32_t* PIO0_18 = (uint32_t*)0x40044048;
static uint32_t* PIO0_19 = (uint32_t*)0x4004404C;

static uint32_t* WDMOD = (uint32_t*)0x40004000;

static uint32_t* ISER0 = (uint32_t*)0xE000E100;

static const uint32_t PIO0_19_RESET_VALUE = 0x00000090;
static const uint32_t PIO0_19_UART_VALUE = 0x00000091;

void ticker_callback() {
    uart1.printf("t\n");
}

int main() {
    uart1.baud(57600);
    uart1.printf("Start the super awesome water temperature sensor reader\n");

    // Initialize LEDs
    DigitalOut led_red(LED_RED);
    DigitalOut led_yellow(LED_YELLOW);
    DigitalOut led_green(LED_GREEN);

    uart1.printf("LEDs initialized\n");

    DigitalOut supply_monitor_enable(SUPPLY_MONITOR_ENABLE);
    AnalogIn supply_monitor_input(SUPPLY_MONITOR_INPUT);
    SupplyMonitor supply_monitor(supply_monitor_input, supply_monitor_enable);

    uart1.printf("SupplyMonitor initialized\n");

    // Initialize DS18B20 sensor
    OneWire one_wire(DS18B20_IO);
    DS18B20 ds18b20(one_wire);

    uart1.printf("DS18B20 initialized\n");

    // Initialize SHT sensor
    I2C i2c_1(SDA, SCL);

    uart1.printf("I2C initialized\n");

    uart1.printf("PIO0_18: %08x\n", *PIO0_18);
    uart1.printf("PIO0_19: %08x\n", *PIO0_19);

    // Initialize the RN2483 module
    RN2483 lora(RN2483_TX, RN2483_RX);
    uint32_t PIO0_19_initialized_value = *PIO0_19;
    *PIO0_19 = PIO0_19_RESET_VALUE;

    uart1.printf("PIO0_18: %08x\n", *PIO0_18);
    uart1.printf("PIO0_19: %08x\n", PIO0_19_initialized_value);

    uart1.printf("WDMOD: %08x\n", *WDMOD);
    uart1.printf("PDAWAKECFG: %08x\n", LPC_SYSCON->PDAWAKECFG);
    uart1.printf("STARTERP1: %08x\n", LPC_SYSCON->STARTERP1);

    // Set up I²C sensor
    i2c_1.frequency(20000);

    uart1.printf("RN2483 initialized\n");

    led_red = 0;
    led_yellow = 0;
    led_green = 0;

    //Ticker ticker;
    //ticker.attach(&ticker_callback, 0.5);
    
    uart1.printf("%08x\n", LPC_SYSCON->SYSAHBCLKCTRL);
    uart1.printf("%08x\n", LPC_SYSCON->MAINCLKSEL);

    wait(0.5);
    //LPC_SYSCON->MAINCLKSEL = 0;
    //LPC_SYSCON->MAINCLKUEN = 1;
    uart1.printf("%08x\n", LPC_SYSCON->MAINCLKSEL);

    wait(0.5);
    disable_unused_peripherals();
    
    uart1.printf("%08x\n", LPC_SYSCON->SYSAHBCLKCTRL);
    
    WakeUp wake_up;
    wake_up.calibrate();
    SleepTimer sleep_timer(wake_up);

    wait(0.5);
    supply_monitor.enable();
    wait(0.5);
    supply_monitor.disable();

    wait(1.0);
    *PIO0_19 = PIO0_19_UART_VALUE;
    lora.sleep(10000);
    wait(0.5);
    //disable_used_peripherals();
    wait(0.5);
    *PIO0_19 = PIO0_19_RESET_VALUE;
    wait(1.0);


    for (;;) {

        //sleep_timer.wait_ms(2000);
 
        NVIC_DisableIRQ(WDT_IRQn);

        LPC_PMU->PCON = 0x1;

        LPC_SYSCON->SYSAHBCLKCTRL |= (1<<15);
        LPC_SYSCON->PDRUNCFG &= ~(1<<1); // Enable IRC oscillator
        LPC_SYSCON->PDRUNCFG &= ~(1<<6); // Enable Watchdog oscillator
        LPC_SYSCON->PDSLEEPCFG &= ~(1<<6); // Enable watchdog in power-down mode

        LPC_SYSCON->MAINCLKSEL = 0;
        LPC_SYSCON->MAINCLKUEN = 0;
        LPC_SYSCON->MAINCLKUEN = 1;

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
        led_green = 0;
        __WFI();
        //sleep();

        enable_used_peripherals();

        led_yellow = 0;
        *PIO0_19 = PIO0_19_RESET_VALUE;
        //sleep();
        uart1.printf("Woke up...\n");
        led_green = 1;
        wait(1.0);
    }

    if (DEV_EUI[0] == 0 && APP_EUI[0] == 0 && APP_KEY[0] == 0)
    {
        for (;;) {
            led_yellow = 1;
            uint8_t buffer[17] = {};

            *PIO0_19 = PIO0_19_UART_VALUE;
            uint8_t bytes = lora.getHWEUI(buffer, 17);
            *PIO0_19 = PIO0_19_RESET_VALUE;

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
        }
    }

    // Join the network
    bool joined = false;
    while (!joined) {
        led_yellow = 1;
        uart1.printf("Joining TTN via OTAA...\n");

        *PIO0_19 = PIO0_19_UART_VALUE;
        joined = lora.initOTAA(DEV_EUI, APP_EUI, APP_KEY, USE_ADR);
        *PIO0_19 = PIO0_19_RESET_VALUE;
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
    while(1) {
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
        uart1.printf(" -> Temp = %u\n", (unsigned)(sht_temp*1000.0f));

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
        uart1.printf(" -> Humi = %u\n", (unsigned)(sht_humi*1000.0f));


        bool timeout = ds18b20.wait_for_completion();
        if (timeout) {
            uart1.printf("Conversion timed out");
        }

        float ds_temp = ds18b20.read_temperature();
        uart1.printf("1-Wire Temp: %u\n", (unsigned)(ds_temp*1000.0f));

        led_green = 0;
        wait(0.2);

        // Measurement done, send it to TTN
        led_yellow = 1;
        uint8_t payload[16] = {};

        float supply_voltage = supply_monitor.get_supply_voltage();

        memcpy(payload, &ds_temp, 4);
        memcpy(payload + 4, &sht_temp, 4);
        memcpy(payload + 8, &sht_humi, 4);
        memcpy(payload + 12, &supply_voltage, 4);

        *PIO0_19 = PIO0_19_UART_VALUE;
        lora.send(1, payload, 16);
        *PIO0_19 = PIO0_19_RESET_VALUE;

        led_yellow = 0;

        wait(INTERVAL);
    }
}
