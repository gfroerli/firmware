#include "mbed.h"
#include <functional>
#include "DS18B20.h"
#include "RN2483.h"

// SHT configuration
const uint8_t SHT3X_I2C_ADDR = 0x45<<1;

// RN2483 configuration, all arrays are in MSB byte order
const uint8_t DEV_EUI[8] = { SET ME };
const uint8_t APP_EUI[8] = { SET ME };
const uint8_t APP_KEY[16] = { SET ME };
const bool USE_ADR = false;

// Measurement interval
const float INTERVAL = 30.0;

float calculate_temp(char msb, char lsb) {
    return -45.0f + 175.0f * (msb<<8 | lsb) / 65535.0f;
}

float calculate_humi(char msb, char lsb) {
    return 100.0f * (msb<<8 | lsb) / 65535.0f;
}

int send_command(I2C& i2c, uint8_t address, uint16_t command) {
    char cmd[2] = {char(command>>8), char(command & 0xFF)};
    return i2c.write(address, cmd, sizeof(cmd));
}

int main() {
    printf("Start the super awesome water temperature sensor reader\n");

    // Initialize LEDs
    DigitalOut led1(LED1);
    DigitalOut led2(LED2);
    DigitalOut led3(LED3);
    DigitalOut led4(LED4);

    // Initialize DS18B20 sensor
    OneWire one_wire(p20);
    DS18B20 ds18b20(one_wire);

    // Initialize SHT sensor
    I2C i2c_0(p28, p27);
    I2C i2c_1(p9, p10);

    // Initialize the RN2483 module
    PinName tx = p13;
    PinName rx = p14;
    RN2483 lora(tx, rx);

    // Set up I²C sensor
    i2c_1.frequency(20000);

    // Join the network
    bool joined = false;
    while (!joined) {
        led4 = 1;
        printf("Joining TTN via OTAA...\n");
        joined = lora.initOTAA(DEV_EUI, APP_EUI, APP_KEY, USE_ADR);
        if (joined) {
            printf("Joined TTN successfully!\n");
        } else {
            printf("Joining TTN failed\n");
        }
        led4 = 0;

        wait(5.0);
    }

    // Main loop
    while(1) {
        printf("------\nStart measurement...\n");

        led1 = 1;
        wait(0.2);

        int error;

        // Start measurement with clock stretching and high repeatability
        error = send_command(i2c_1, SHT3X_I2C_ADDR, 0x2C06);
        if (error) {
            printf("i2c.write failed: %i\n", error);
        }

        // Start conversion
        ds18b20.start_measurement();
        wait(0.5);

        char data[6] = {};
        error = i2c_1.read(SHT3X_I2C_ADDR, data, 6);
        if (error) {
            printf("i2c_1.read failed: %i\n", error);
        }

        for(int i=0; i<6; ++i) {
            printf("%02x", data[i]);
        }
        float sht_temp = calculate_temp(data[0], data[1]);
        printf(" -> Temp = %.2f", sht_temp);

        float sht_humi = calculate_humi(data[3], data[4]);
        printf(" Humi = %.2f\n", sht_humi);

        bool timeout = ds18b20.wait_for_completion();
        if (timeout) {
            printf("Conversion timed out");
        }

        float ds_temp = ds18b20.read_temperature();
        printf("1-Wire Temp %.2f\n", ds_temp);

        led1 = 0;
        wait(0.2);

        // Measurement done, send it to TTN
        led2 = 1;
        uint8_t payload[12];
        memcpy(payload, &ds_temp, 4);
        memcpy(payload + 4, &sht_temp, 4);
        memcpy(payload + 8, &sht_humi, 4);
        lora.send(1, payload, 12);
        led2 = 0;

        wait(INTERVAL);
    }
}
