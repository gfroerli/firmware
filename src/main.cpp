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
const float INTERVAL = 30.0;

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

static const uint32_t PIO0_19_RESET_VALUE = 0x00000090;
static const uint32_t PIO0_19_UART_VALUE = 0x00000091;

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

    // Set up I²C sensor
    i2c_1.frequency(20000);

    uart1.printf("RN2483 initialized\n");

    led_red = 0;
    led_yellow = 0;
    led_green = 0;

    do {
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
    } while (DEV_EUI[0] == 0 && APP_EUI[0] == 0 && APP_KEY[0] == 0);

    // Join the network
    *PIO0_19 = PIO0_19_UART_VALUE;
    bool joined = lora.isJoined();
    *PIO0_19 = PIO0_19_RESET_VALUE;
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
