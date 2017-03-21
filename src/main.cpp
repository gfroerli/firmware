#include "mbed.h"
#include <functional>
#include "DS18B20.h"

uint8_t SHT3X_I2C_ADDR = 0x45<<1;

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

    DigitalOut led1(LED1);
    DigitalOut led2(LED2);

    I2C i2c_0(p28, p27);
    I2C i2c_1(p9, p10);

    OneWire one_wire(p20);
    DS18B20 ds18b20(one_wire);

    using I2CLink = std::reference_wrapper<I2C>;

    I2CLink i2cs[2] = {i2c_0, i2c_1};

    for (auto i2c : i2cs) {
        i2c.get().frequency(20000);
    }

    while(1) {
        led1 = 1;
        wait(0.2);

        for (auto i2c : i2cs) {
            // Start measurement with clock stretching and high repeatability
            int error = send_command(i2c, SHT3X_I2C_ADDR, 0x2C06);
            if (error) {
                printf("i2c.write failed: %i\n", error);
            }
        }
        // start conversion
        ds18b20.start_measurement();
        wait(0.5);

        for (auto i2c : i2cs) {
            char data[6] = {};
            int error = i2c.get().read(SHT3X_I2C_ADDR, data, 6);
            if (error) {
                printf("i2c.get().read failed: %i\n", error);
            }

            for(int i=0; i<6; ++i) {
                printf("%02x", data[i]);
            }
            float tmp = calculate_temp(data[0], data[1]);
            printf(" -> Temp = %.2f", tmp);

            float humi = calculate_humi(data[3], data[4]);
            printf(" Humi = %.2f\n", humi);
        }

        bool timeout = ds18b20.wait_for_completion();
        if (timeout) {
            printf("Conversion timed out");
        }

        float temperature = ds18b20.read_temperature();
        printf("1-Wire Temp %.2f\n", temperature);

        led1 = 0;
        wait(0.2);
    }
}
