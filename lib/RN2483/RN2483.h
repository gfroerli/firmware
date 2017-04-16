/*
* Copyright (c) 2016 Dan Knox. All rights reserved.
*
* This file is part of RN2483.
*
* RN2483 is free software: you can redistribute it and/or modify
* it under the terms of the GNU Lesser General Public License as
* published by the Free Software Foundation, either version 3 of
* the License, or(at your option) any later version.
*
* RN2483 is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
* GNU Lesser General Public License for more details.
*
* You should have received a copy of the GNU Lesser General Public
* License along with RN2483.  If not, see
* <http://www.gnu.org/licenses/>.
*/

#ifndef _RN2483_h
#define _RN2483_h

#include "mbed.h"
#include <stdint.h>

//#define USE_DYNAMIC_BUFFER

#define DEFAULT_INPUT_BUFFER_SIZE 64
#define DEFAULT_RECEIVED_PAYLOAD_BUFFER_SIZE 32
#define DEFAULT_TIMEOUT 120
#define RECEIVE_TIMEOUT 60000
#define DEFAULT_FSB 2
#define DEFAULT_PWR_IDX_868 1
#define DEFAULT_PWR_IDX_915 5
#define DEFAULT_SF_868 7
#define DEFAULT_SF_915 7

#define ENABLE_SLEEP

// Available error codes.
enum MacTransmitErrorCodes {
    NoError = 0,
    NoResponse = 1,
    Timedout = 2,
    PayloadSizeError = 3,
    InternalError = 4,
    Busy = 5,
    NetworkFatalError = 6,
    NotConnected = 7,
    NoAcknowledgment = 8,
    Silent = 9,
};

// Provides a simple, abstracted interface to Microchip's RN2483 LoRaWAN module.

class RN2483
{
public:
    
    /**
    * @brief Create a new instance of the RN2483.
    * @param Serial TX pin name.
    * @param Serial RX pin name.
    */
    RN2483(PinName tx, PinName rx);

    /**
    * @return Returns the default device baud rate.
    */
    uint32_t getDefaultBaudRate() {
        return 57600;
    };

    /**
    * @brief Initialise settings and connect to network using Over The Air activation.
    * @param devEUI provided by LoRaWAN Network server registration.
    * @param appEUI provided by LoRaWAN Network server registration.
    * @param appKey provided by LoRaWAN Network server registration.
    * @return Returns true if network confirmation and able to save settings.
    */
    bool initOTA(const uint8_t devEUI[8], const uint8_t appEUI[8], const uint8_t appKey[16], bool adr = true);

    /**
    * @brief Initializes the device and connects to the network using Activation By Personalization.
    * @param devADDR provided by LoRaWAN Network server registration.
    * @param appSKey provided by LoRaWAN Network server registration.
    * @param nwkSKey provided by LoRaWAN Network server registration.
    * @return Returns true if the parameters were valid and able to save settings.
    */
    bool initABP(const uint8_t devAddr[4], const uint8_t appSKey[16], const uint8_t nwkSKey[16], bool adr = true);

    /**
    * @brief Attempts to connect to the network using Over The Air Activation.
    * @return Returns true if able to join network.
    */
    bool joinOTAA();
    
    /**
    * @brief Attempts to connect to the network using Activation By Personalization.
    * @return Returns true if able to join network.
    */
    bool joinABP();
    
    /**
    * @brief Sends the given payload without acknowledgement.
    * @param Port to use for transmission.
    * @param Payload buffer
    * @param Payload buffer size
    * @return Returns 0 (NoError) when data was sucessfully fowarded to radio, otherwise returns MacTransmitErrorCode.
    */
    uint8_t send(uint8_t port, const uint8_t* payload, uint8_t size);

    /**
    * @brief Sends the given payload with acknowledgement.
    * @param Port to use for transmission.
    * @param Payload buffer
    * @param Payload buffer size
    * @param Number of transmission retries in event of network transmission failure.
    * @return Returns 0 (NoError) when network acks transmission, otherwise returns MacTransmitErrorCode.
    */
    uint8_t sendReqAck(uint8_t port, const uint8_t* payload, uint8_t size, uint8_t maxRetries);

    /**
    * @brief Copies the latest received packet (optionally starting from the "payloadStartPosition" of the payload).
    * @param Buffer to read into.
    * @param Buffer size.
    * @return Returns the number of bytes written or 0 if no packet is received since last transmission.
    */
    uint16_t receive(uint8_t* buffer, uint16_t size, uint16_t payloadStartPosition = 0);

    /**
    * @brief Gets the preprogrammed EUI node address from the module in HEX.
    * @param Buffer to read into.
    * @param Buffer size.
    * @return Returns the number of bytes written or 0 in case of error..
    */
    uint8_t getHWEUI(uint8_t* buffer, uint8_t size);
    
   /**
    * @brief Informs the RN2483 to do an ADC conversion on the VDD.
    * @param Pass pointer to long for conversion to read into.
    * @return Returns if a value was sucessfully read into the long.
    */
    bool getVDD(long *vdd);

    /**
    * @brief Enables all the channels that belong to the given Frequency Sub-Band (FSB)
    * disables the rest.
    * @param FSB is [1, 8] or 0 to enable all channels.
    * @return Returns true if all channels were set successfully.
    */
    bool setFsbChannels(uint8_t fsb);

    /**
    * @brief Sets the spreading factor.
    * @param Spreading factor parameter.
    * @return Returns true if was set successfully.
    */
    bool setSpreadingFactor(uint8_t spreadingFactor);

    /**
    * @brief Sets the power index
    * @param 868MHz: 1 to 5 / 915MHz: 5, 7, 8, 9 or 10.
    * @return Returns true if succesful.
    */
    bool setPowerIndex(uint8_t powerIndex);

    /**
    * @brief Sets the time interval for the link check process. When the time expires, the next application
    * packet will include a link check command to the server.
    * @param Decimal number that sets the time interval in seconds, from 0 to 65535. 0 disables link check process.
    * @return Returns true if parameter is valid or false if time interval is not valid.
    */
    bool setLinkCheckInterval(uint8_t linkCheckInterval);

    /**
    * @brief Sets the battery level required for the Device Status Answer frame in LoRaWAN Class A Protocol.
    * @param temperature Decimal number between 0-255 representing battery level. 0 means external power, 1 means
    * low level, 254 means high level, 255 means the device was unable to measure battery level.
    * @return Returns true if battery level is valid or false if value not valid.
    */
    bool setBattery(uint8_t batLvl);

    /**
    * @brief Sets the module operation frequency on a given channel ID.
    * @param Channel ID from 3 - 15.
    * @param Decimal number representing the frequency.
    * 863000000 to 870000000 or 433050000 to 434790000 in Hz
    * @return Returns true if parameters are valid or false if not.
    */
    bool setChannelFreq(uint8_t channelID, uint32_t frequency);
    
    /**
    * @brief Sets the duty cycle allowed on the given channel ID.
    * @param Channel ID to set duty cycle (0-15),
    * @param Duty cycle is 0 - 100% as a float.
    * @return Returns true if parameters are valid or false if not.
    */
    bool setDutyCycle(uint8_t channelID, float dutyCycle);

    /**
    * @brief Sets the data rate for a given channel ID.
    * Please refer to the LoRaWAN spec for the actual values.
    * @param Channel ID from 0 - 15.
    * @param Number representing the minimum data rate range from 0 to 7.
    * @param Number representing the maximum data rate range from 0 to 7
    * @return Returns true if parameters are valid or false if not.
    */
    bool setDrRange(uint8_t channelID, uint8_t minRange, uint8_t maxRange);
    
    /**
    * @brief Sets a given channel ID to be enabled or disabled.
    * @param Channel ID from 0 - 15.
    * @param Flag representing if channel is enabled or disabled.
    * Warning: duty cycle, frequency and data range must be set for a channel
    * before enabling!
    * @return Returns true if parameters are valid or false if not.
    */
    bool setStatus(uint8_t channelID, bool status);
    
    /**
    * @brief The network can issue a command to silence the RN2483. This restores the module. 
    * @return Returns true if parameters are valid or false if not.
    */
    bool forceEnable();
    
    /**
    * @brief Saves configurable parameters to eeprom. 
    * @return Returns true if parameters are valid or false if not.
    */
    bool saveConfiguration();
    
    /**
    * @brief Sends the command together with the given, paramValue (optional)
    * @param Command should include a trailing space if paramValue is set. Refer to RN2483 command ref
    * @param Command Parameter to send
    * @param Size of param buffer
    * @return Returns true on success or false if invalid.
    */
    bool sendCommand(const char* command, const uint8_t* paramValue, uint16_t size);
    bool sendCommand(const char* command, uint8_t paramValue);
    bool sendCommand(const char* command, const char* paramValue = NULL);

    /**
    * @brief Sends the command together with the given paramValue (optional)
    * @param MAC param should include a trailing space if paramValue is set. Refer to RN2483 command ref.
    * @param Param value to send
    * @param Size of Param buffer
    * @return Returns true on success or false if invalid.
    */
    bool setMacParam(const char* paramName, const uint8_t* paramValue, uint16_t size);
    bool setMacParam(const char* paramName, uint8_t paramValue);
    bool setMacParam(const char* paramName, const char* paramValue);

#ifdef ENABLE_SLEEP
    /**
    * @brief Sends a serial line break to wake up the RN2483
    */
    void wakeUp();

    /**
    * @brief Sends the RN2483 to sleep for a finite length of time.
    * @param Milliseconds to sleep for, range is 100 to 4294967295
    */
    void sleep(uint32_t);
    
    /**
    * @brief Sends the RN2483 to sleep for a finite length of time.
    * Roughly three days.
    */
    void sleep();
    
#endif

#ifdef USE_DYNAMIC_BUFFER
    // Sets the size of the input buffer.
    // Needs to be called before initOTA()/initABP().
    void setInputBufferSize(uint16_t value) {
        this->inputBufferSize = value;
    };

    // Sets the size of the "Received Payload" buffer.
    // Needs to be called before initOTA()/initABP().
    void setReceivedPayloadBufferSize(uint16_t value) {
        this->receivedPayloadBufferSize = value;
    };
#endif

private:

    Serial _RN2483;

    // The size of the input buffer. Equals DEFAULT_INPUT_BUFFER_SIZE
    // by default or (optionally) a user-defined value when using USE_DYNAMIC_BUFFER.
    uint16_t inputBufferSize;

    // The size of the received payload buffer. Equals DEFAULT_RECEIVED_PAYLOAD_BUFFER_SIZE
    // by default or (optionally) a user-defined value when using USE_DYNAMIC_BUFFER.
    uint16_t receivedPayloadBufferSize;

    // Flag used to make sure the received payload buffer is
    // current with the latest transmission.
    bool packetReceived;

    // Used to distinguise between RN2483 and RN2903.
    // Currently only being set during reset().
    bool isRN2903;

#ifdef USE_DYNAMIC_BUFFER
    // Flag to make sure the buffers are not allocated more than once.
    bool isBufferInitialized;

    char* inputBuffer;
    char* receivedPayloadBuffer;
#else
    char inputBuffer[DEFAULT_INPUT_BUFFER_SIZE];
    char receivedPayloadBuffer[DEFAULT_RECEIVED_PAYLOAD_BUFFER_SIZE];
#endif

    /**
    * @brief Takes care of the init tasks common to both initOTA() and initABP.
    */
    inline void init();

    /**
    * @brief Reads a line from the device serial stream.
    * @param Buffer to read into.
    * @param Size of buffer.
    * @param Position to start from.
    * @return Number of bytes read.
    */
    uint16_t readLn(char* buffer, uint16_t size, uint16_t start = 0);

    /**
    * @brief Reads a line from the input buffer
    * @return Number of bytes read.
    */
    uint16_t readLn() {
        return readLn(this->inputBuffer, this->inputBufferSize);
    };

    /**
    * @brief Waits for the given string.
    * @param String to look for.
    * @param Timeout Period
    * @param Position to start from.
    * @return Returns true if the string is received before a timeout.
    * Returns false if a timeout occurs or if another string is received.
    */
    bool expectString(const char* str, uint16_t timeout = DEFAULT_TIMEOUT);

    /**
    * @brief Looks for an 'OK' response from the RN2483
    * @param Timeout Period
    * @return Returns true if the string is received before a timeout.
    * Returns false if a timeout occurs or if another string is received.
    */
    bool expectOK(uint16_t timeout = DEFAULT_TIMEOUT);

    /**
    * @brief Sends a reset command to the module
    * Also sets-up some initial parameters like power index, SF and FSB channels.
    * @return Waits for sucess reponse or timeout.
    */
    bool resetDevice();

    /**
    * @brief Sends a join command to the network
    * @param Type of join, OTAA or ABP
    * @return Returns true on success or false if fail.
    */
    bool joinNetwork(const char* type);

    /**
    * @brief Returns the enum that is mapped to the given "error" message
    * @param Error to lookup.
    * @return Returns the enum
    */
    uint8_t lookupMacTransmitError(const char* error);

    /**
    * @brief Sends a a payload and blocks until there is a response back,
    * or the receive windows have closed or the hard timeout has passed.
    * @param Transmit type
    * @param Port to use for transmit
    * @param Payload buffer
    * @param Size of payload buffer
    * @return Returns if sucessfull or if a MAC transmit error.
    */
    uint8_t macTransmit(const char* type, uint8_t port, const uint8_t* payload, uint8_t size);

    /**
    * @brief Parses the input buffer and copies the received payload into
    * the "received payload" buffer when a "mac rx" message has been received.
    * @return Returns 0 (NoError) or otherwise one of the MacTransmitErrorCodes.
    */
    uint8_t onMacRX();

    /**
    * @brief Private method to read serial port with timeout
    * @param The time to wait for in milliseconds.
    * @return Returns character or -1 on timeout
    */
    int timedRead(int _timeout);

    /**
    * @brief Read characters into buffer.
    * Terminates if length characters have been read, timeout, or
    * if the terminator character has been detected
    * @param The terminator character to look for
    * @param The buffer to read into.
    * @param The size of the buffer.
    * @return The number of bytes read. 0 means no valid data found.
    */
    size_t readBytesUntil(char terminator, char *buffer, size_t length);
};

#endif // RN2483
