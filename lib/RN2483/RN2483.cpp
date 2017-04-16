#include "RN2483.h"
#include "StringLiterals.h"
#include "Utils.h"
#include "errno.h"
#include "limits.h"

// Structure for mapping error response strings and error codes.
typedef struct StringEnumPair {
    const char* stringValue;
    uint8_t enumValue;
} StringEnumPair_t;

/**
* @brief Create a new instance of the RN2483.
* @param Serial TX pin name.
* @param Serial RX pin name.
*/
RN2483::RN2483(PinName tx, PinName rx) :
    _RN2483(tx, rx, getDefaultBaudRate()),
    inputBufferSize(DEFAULT_INPUT_BUFFER_SIZE),
    receivedPayloadBufferSize(DEFAULT_RECEIVED_PAYLOAD_BUFFER_SIZE),
    packetReceived(false),
    isRN2903(false)
{
#ifdef USE_DYNAMIC_BUFFER
    this->isBufferInitialized = false;
#endif
}

/**
* @brief Takes care of the init tasks common to both initOTAA() and initABP.
*/
void RN2483::init()
{
#ifdef USE_DYNAMIC_BUFFER
    // make sure the buffers are only initialized once
    if (!isBufferInitialized) {
        this->inputBuffer = static_cast<char*>(malloc(this->inputBufferSize));
        this->receivedPayloadBuffer = static_cast<char*>(malloc(this->receivedPayloadBufferSize));
        isBufferInitialized = true;
    }
#endif
    // make sure the module's state is synced and woken up
    sleep(259200000);
    wait_ms(10);
    wakeUp();
}

/**
* @brief Initialise settings and connect to network using Over The Air activation.
* @param devEUI provided by LoRaWAN Network server registration.
* @param appEUI provided by LoRaWAN Network server registration.
* @param appKey provided by LoRaWAN Network server registration.
* @return Returns true if network confirmation and able to save settings.
*/
bool RN2483::initOTAA(const uint8_t devEUI[8], const uint8_t appEUI[8], const uint8_t appKey[16], bool adr)
{
    init();
    if(resetDevice() && setMacParam(STR_DEV_EUI, devEUI, 8) && setMacParam(STR_APP_EUI, appEUI, 8) &&
            setMacParam(STR_APP_KEY, appKey, 16) && setMacParam(STR_ADR, BOOL_TO_ONOFF(adr)) && joinOTAA()) {
        if(saveConfiguration()) {
            return true;
        }
    }
    return false;
}

/**
* @brief Initializes the device and connects to the network using Activation By Personalization.
* @param devADDR provided by LoRaWAN Network server registration.
* @param appSKey provided by LoRaWAN Network server registration.
* @param nwkSKey provided by LoRaWAN Network server registration.
* @return Returns true if the parameters were valid and able to save settings.
*/
bool RN2483::initABP(const uint8_t devAddr[4], const uint8_t appSKey[16], const uint8_t nwkSKey[16], bool adr)
{
    init();
    if(resetDevice() && setMacParam(STR_DEV_ADDR, devAddr, 4) && setMacParam(STR_APP_SESSION_KEY, appSKey, 16) &&
            setMacParam(STR_NETWORK_SESSION_KEY, nwkSKey, 16) && setMacParam(STR_ADR, BOOL_TO_ONOFF(adr)) &&
            joinABP()) {
        if(saveConfiguration()) {
            return true;
        }
    }
    return false;
}

/**
* @brief Attempts to connect to the network using Over The Air Activation.
* @return Returns true if able to join network.
*/
bool RN2483::joinOTAA()
{
    return joinNetwork(STR_OTAA);
}

/**
* @brief Attempts to connect to the network using Activation By Personalization.
* @return Returns true if able to join network.
*/
bool RN2483::joinABP()
{
    return joinNetwork(STR_ABP);
}

/**
* @brief Sends the given payload without acknowledgement.
* @param Port to use for transmission.
* @param Payload buffer
* @param Payload buffer size
* @return Returns 0 (NoError) when data was sucessfully fowarded to radio, otherwise returns MacTransmitErrorCode.
*/
uint8_t RN2483::send(uint8_t port, const uint8_t* payload, uint8_t size)
{
    return macTransmit(STR_UNCONFIRMED, port, payload, size);
}

/**
* @brief Sends the given payload with acknowledgement.
* @param Port to use for transmission.
* @param Payload buffer
* @param Payload buffer size
* @param Number of transmission retries in event of network transmission failure.
* @return Returns 0 (NoError) when network acks transmission, otherwise returns MacTransmitErrorCode.
*/
uint8_t RN2483::sendReqAck(uint8_t port, const uint8_t* payload, uint8_t size, uint8_t maxRetries)
{
    // Need to implement retries! mac set retx
    return macTransmit(STR_CONFIRMED, port, payload, size);
}

/**
* @brief Copies the latest received packet (optionally starting from the "payloadStartPosition" of the payload).
* @param Buffer to read into.
* @param Buffer size.
* @return Returns the number of bytes written or 0 if no packet is received since last transmission.
*/
uint16_t RN2483::receive(uint8_t* buffer, uint16_t size,
                         uint16_t payloadStartPosition)
{

    if (!this->packetReceived) {
        return 0;
    }

    uint16_t inputIndex = payloadStartPosition * 2; // payloadStartPosition is in bytes, not hex char pairs
    uint16_t outputIndex = 0;

    // check that the asked starting position is within bounds
    if (inputIndex >= this->receivedPayloadBufferSize) {
        return 0;
    }

    // stop at the first string termination char, or if output buffer is over, or if payload buffer is over
    while (outputIndex < size
            && inputIndex + 1 < this->receivedPayloadBufferSize
            && this->receivedPayloadBuffer[inputIndex] != 0
            && this->receivedPayloadBuffer[inputIndex + 1] != 0) {
        buffer[outputIndex] = HEX_PAIR_TO_BYTE(
                                  this->receivedPayloadBuffer[inputIndex],
                                  this->receivedPayloadBuffer[inputIndex + 1]);

        inputIndex += 2;
        outputIndex++;
    }

    // Note: if the payload has an odd length, the last char is discarded

    buffer[outputIndex] = 0; // terminate the string

    return outputIndex;
}

/**
* @brief Gets the preprogrammed EUI node address from the module in HEX.
* @param Buffer to read into.
* @param Buffer size.
* @return Returns the number of bytes written or 0 in case of error..
*/
uint8_t RN2483::getHWEUI(uint8_t* buffer, uint8_t size)
{
    _RN2483.printf(STR_CMD_GET_HWEUI);
    _RN2483.printf(CRLF);

    // TODO move to general "read hex" method
    uint8_t inputIndex = 0;
    uint8_t outputIndex = 0;
    Timer t;
    t.start();

    int start = t.read_ms ();
    while (t.read_ms () < start + DEFAULT_TIMEOUT) {
        if (readLn() > 0) {
            while (outputIndex < size
                    && inputIndex + 1 < this->inputBufferSize
                    && this->inputBuffer[inputIndex] != 0
                    && this->inputBuffer[inputIndex + 1] != 0) {
                buffer[outputIndex] = HEX_PAIR_TO_BYTE(
                                          this->inputBuffer[inputIndex],
                                          this->inputBuffer[inputIndex + 1]);
                inputIndex += 2;
                outputIndex++;
            }
            t.stop();
            return outputIndex;
        }
    }
    t.stop();
    return 0;
}

/**
* @brief Informs the RN2483 to do an ADC conversion on the VDD.
* @param Pass pointer to long for conversion to read into.
* @return Returns if a value was sucessfully read into the long.
*/
bool RN2483::getVDD(long *vdd)
{
    _RN2483.printf(STR_CMD_GET_VDD);
    _RN2483.printf(CRLF);
    Timer t;
    t.start();
    int timeout = t.read_ms() + RECEIVE_TIMEOUT; // hard timeouts
    while (t.read_ms() < timeout) {
        if (readLn() > 0) {
            char *temp;
            bool rc = true;
            errno = 0;
            *vdd = strtol(this->inputBuffer, &temp, 10);
            if (temp == this->inputBuffer || *temp != '\0' || ((*vdd == LONG_MIN || 
            *vdd == LONG_MAX) && errno == ERANGE)){
                rc = false;
            }
            t.stop();
            return rc;
        }
    }
    t.stop();
    return false;
}

#ifdef ENABLE_SLEEP
/**
* @brief Sends a serial line break to wake up the RN2483
*/
void RN2483::wakeUp()
{
   // "emulate" break condition
    _RN2483.send_break();
    // set baudrate
    _RN2483.baud(getDefaultBaudRate());
    _RN2483.putc((uint8_t)0x55);
}

/**
* @brief Sends the RN2483 to sleep for a finite length of time.
* @param Milliseconds to sleep for.
*/
void RN2483::sleep(uint32_t sleepLength)
{
    if(sleepLength > 100) {
        _RN2483.printf("%s%u",STR_CMD_SLEEP,sleepLength);
        _RN2483.printf(CRLF);
    }
}

/**
* @brief Sends the RN2483 to sleep for a finite length of time.
* Roughly three days.
*/
void RN2483::sleep()
{
    sleep(4294967295);
}

#endif

/**
* @brief Reads a line from the device serial stream.
* @param Buffer to read into.
* @param Size of buffer.
* @param Position to start from.
* @return Number of bytes read.
*/
uint16_t RN2483::readLn(char* buffer, uint16_t size, uint16_t start)
{
    int len = readBytesUntil('\n', buffer + start, size);
    if (len > 0) {
        this->inputBuffer[start + len - 1] = 0; // bytes until \n always end with \r, so get rid of it (-1)
    }

    return len;
}

/**
* @brief Waits for the given string.
* @param String to look for.
* @param Timeout Period
* @param Position to start from.
* @return Returns true if the string is received before a timeout.
* Returns false if a timeout occurs or if another string is received.
*/
bool RN2483::expectString(const char* str, uint16_t timeout)
{
    Timer t;
    t.start();
    int start = t.read_ms();
    while (t.read_ms() < start + timeout) {
        if (readLn() > 0) {
            if (strstr(this->inputBuffer, str) != NULL) {
                t.stop();
                return true;
            }
            t.stop();
            return false;
        }
    }
    t.stop();
    return false;
}

/**
* @brief Looks for an 'OK' response from the RN2483
* @param Timeout Period
* @return Returns true if the string is received before a timeout.
* Returns false if a timeout occurs or if another string is received.
*/
bool RN2483::expectOK(uint16_t timeout)
{
    return expectString(STR_RESULT_OK, timeout);
}

/**
* @brief Sends a reset command to the module
* Also sets-up some initial parameters like power index, SF and FSB channels.
* @return Waits for sucess reponse or timeout.
*/
bool RN2483::resetDevice()
{
    _RN2483.printf(STR_CMD_RESET);
    _RN2483.printf(CRLF);
    if (expectString(STR_DEVICE_TYPE_RN)) {
        if (strstr(this->inputBuffer, STR_DEVICE_TYPE_RN2483) != NULL) {
            isRN2903 = false;
            return setPowerIndex(DEFAULT_PWR_IDX_868) &&
                   setSpreadingFactor(DEFAULT_SF_868);
        } else if (strstr(this->inputBuffer, STR_DEVICE_TYPE_RN2903) != NULL) {
            // TODO move into init once it is decided how to handle RN2903-specific operations
            isRN2903 = true;

            return setFsbChannels(DEFAULT_FSB) &&
                   setPowerIndex(DEFAULT_PWR_IDX_915) &&
                   setSpreadingFactor(DEFAULT_SF_915);
        } else {
            return false;
        }
    }
    return false;
}

/**
* @brief Enables all the channels that belong to the given Frequency Sub-Band (FSB)
* disables the rest.
* @param FSB is [1, 8] or 0 to enable all channels.
* @return Returns true if all channels were set successfully.
*/
bool RN2483::setFsbChannels(uint8_t fsb)
{
    uint8_t first125kHzChannel = fsb > 0 ? (fsb - 1) * 8 : 0;
    uint8_t last125kHzChannel = fsb > 0 ? first125kHzChannel + 7 : 71;
    uint8_t fsb500kHzChannel = fsb + 63;

    bool allOk = true;
    for (uint8_t i = 0; i < 72; i++) {
        _RN2483.printf(STR_CMD_SET_CHANNEL_STATUS);
        _RN2483.printf("%u",i);
        _RN2483.printf(" ");
        _RN2483.printf(BOOL_TO_ONOFF(((i == fsb500kHzChannel) || (i >= first125kHzChannel && i <= last125kHzChannel))));
        _RN2483.printf(CRLF);

        allOk &= expectOK();
    }

    return allOk;
}

/**
* @brief Sets the spreading factor.
* @param Spreading factor parameter.
* @return Returns true if was set successfully.
*/
bool RN2483::setSpreadingFactor(uint8_t spreadingFactor)
{
    int8_t datarate;
    if (!isRN2903) {
        // RN2483 SF(DR) = 7(5), 8(4), 9(3), 10(2), 11(1), 12(0)
        datarate = 12 - spreadingFactor;
    } else {
        // RN2903 SF(DR) = 7(3), 8(2), 9(1), 10(0)
        datarate = 10 - spreadingFactor;
    }

    if (datarate > -1) {
        return setMacParam(STR_DATARATE, datarate);
    }

    return false;
}

/**
* @brief Sets the power index
* @param 868MHz: 1 to 5 / 915MHz: 5, 7, 8, 9 or 10.
* @return Returns true if succesful.
*/
bool RN2483::setPowerIndex(uint8_t powerIndex)
{
    return setMacParam(STR_PWR_IDX, powerIndex);
}

/**
* @brief Sets the time interval for the link check process. When the time expires, the next application
* packet will include a link check command to the server.
* @param Decimal number that sets the time interval in seconds, from 0 to 65535. 0 disables link check process.
* @return Returns true if parameter is valid or false if time interval is not valid.
*/
bool RN2483::setLinkCheckInterval(uint8_t linkCheckInterval)
{
    return setMacParam(STR_LNK_CHK, linkCheckInterval);
}

/**
* @brief Sets the battery level required for the Device Status Answer frame in LoRaWAN Class A Protocol.
* @param temperature Decimal number between 0-255 representing battery level. 0 means external power, 1 means
* low level, 254 means high level, 255 means the device was unable to measure battery level.
* @return Returns true if battery level is valid or false if value not valid.
*/
bool RN2483::setBattery(uint8_t batLvl)
{
    return setMacParam(STR_BAT, batLvl);
}

/**
* @brief Sets the module operation frequency on a given channel ID.
* @param Channel ID from 3 - 15.
* @param Decimal number representing the frequency.
* 863000000 to 870000000 or 433050000 to 434790000 in Hz
* @return Returns true if parameters are valid or false if not.
*/
bool RN2483::setChannelFreq(uint8_t channelID, uint32_t frequency)
{
    if((channelID <= 15 && channelID >= 3)) {
        if((frequency <=870000000 && frequency >= 863000000)||(frequency <=434790000 && frequency >= 433050000)) {
            char buffer [15];
            int bytesWritten = sprintf(buffer, "%d %lu", channelID, frequency);
            // Check to make sure sprintf did not return an error before sending.
            if(bytesWritten > 0) {
                return setMacParam(STR_CH_FREQ, buffer);
            }
        }
    }
    return false;
}

/**
* @brief Sets the duty cycle allowed on the given channel ID.
* @param Channel ID to set duty cycle (0-15),
* @param Duty cycle is 0 - 100% as a float.
* @return Returns true if parameters are valid or false if not.
*/
bool RN2483::setDutyCycle(uint8_t channelID, float dutyCycle)
{
    // Convert duty cycle into the required value using equation (100 / X) - 1
    if((dutyCycle <= (float)100 && dutyCycle >=(float)0) && (channelID > 15)) {
        uint8_t dutyCycleSetting = ((float)100 / dutyCycle) - 1;
        // Create the string for the settings
        char buffer [15];
        int bytesWritten = sprintf(buffer, "%d %d", channelID, dutyCycleSetting);
        // Check to make sure sprintf did not return an error before sending.
        if(bytesWritten > 0) {
            return setMacParam(STR_CH_DCYCLE, buffer);
        }
    }
    return false;
}

/**
* @brief Sets the data rate for a given channel ID.
* Please refer to the LoRaWAN spec for the actual values.
* @param Channel ID from 0 - 15.
* @param Number representing the minimum data rate range from 0 to 7.
* @param Number representing the maximum data rate range from 0 to 7
* @return Returns true if parameters are valid or false if not.
*/
bool RN2483::setDrRange(uint8_t channelID, uint8_t minRange, uint8_t maxRange)
{
    if((channelID <= 15)&&(minRange<=7)&&(maxRange<=7)) {
        char buffer [15];
        int bytesWritten = sprintf(buffer, "%d %d %d", channelID, minRange, maxRange);
        // Check to make sure sprintf did not return an error before sending.
        if(bytesWritten > 0) {
            return setMacParam(STR_CH_DRRANGE, buffer);
        }
    }
    return false;
}

/**
* @brief Sets a given channel ID to be enabled or disabled.
* @param Channel ID from 0 - 15.
* @param Flag representing if channel is enabled or disabled.
* Warning: duty cycle, frequency and data range must be set for a channel
* before enabling!
* @return Returns true if parameters are valid or false if not.
*/
bool RN2483::setStatus(uint8_t channelID, bool status)
{
    if((channelID <= 15)) {
        int bytesWritten = 0;
        char buffer [15];
        if(status)
            bytesWritten = sprintf(buffer, "%d %s", channelID, "on");
        else {
            bytesWritten = sprintf(buffer, "%d %s", channelID, "off");
        }
        // Check to make sure sprintf did not return an error before sending.
        if(bytesWritten > 0) {
            return sendCommand(STR_CMD_SET_CHANNEL_STATUS, buffer);
        }
    }
    return false;
}

/**
* @brief The network can issue a command to silence the RN2483. This restores the module.
* @return Returns true if parameters are valid or false if not.
*/
bool RN2483::forceEnable()
{
    return sendCommand(STR_MAC_FORCEENABLE);
}

/**
* @brief Saves configurable parameters to eeprom.
* @return Returns true if parameters are valid or false if not.
*/
bool RN2483::saveConfiguration()
{
    // Forced to return true currently.
    // Currently broken due to the long length of time it takes save to return.
    //_RN2483.printf(STR_CMD_SAVE);
    //_RN2483.printf(CRLF);
    return true;
}

/**
* @brief Sends the command together with the given, paramValue (optional)
* @param Command should include a trailing space if paramValue is set. Refer to RN2483 command ref
* @param Command Parameter to send
* @param Size of param buffer
* @return Returns true on success or false if invalid.
*/
bool RN2483::sendCommand(const char* command, const uint8_t* paramValue, uint16_t size)
{
    _RN2483.printf(command);

    for (uint16_t i = 0; i < size; ++i) {
        _RN2483.putc(static_cast<char>(NIBBLE_TO_HEX_CHAR(HIGH_NIBBLE(paramValue[i]))));
        _RN2483.putc(static_cast<char>(NIBBLE_TO_HEX_CHAR(LOW_NIBBLE(paramValue[i]))));
    }

    _RN2483.printf(CRLF);

    return expectOK();
}

/**
* @brief Sends the command together with the given, paramValue (optional)
* @param Command should include a trailing space if paramValue is set. Refer to RN2483 command ref
* @param Command Parameter to send
* @return Returns true on success or false if invalid.
*/
bool RN2483::sendCommand(const char* command, uint8_t paramValue)
{
    _RN2483.printf(command);
    _RN2483.printf("%u",paramValue);
    _RN2483.printf(CRLF);

    return expectOK();
}

/**
* @brief Sends the command together with the given, paramValue (optional)
* @param Command should include a trailing space if paramValue is set. Refer to RN2483 command ref
* @param Command Parameter to send
* @return Returns true on success or false if invalid.
*/
bool RN2483::sendCommand(const char* command, const char* paramValue)
{
    _RN2483.printf(command);
    if (paramValue != NULL) {
        _RN2483.printf(paramValue);
    }
    _RN2483.printf(CRLF);
    return expectOK();
}

/**
* @brief Sends a join command to the network
* @param Type of join, OTAA or ABP
* @return Returns true on success or false if fail.
*/
bool RN2483::joinNetwork(const char* type)
{
    _RN2483.printf(STR_CMD_JOIN);
    _RN2483.printf(type);
    _RN2483.printf(CRLF);

    return expectOK() && expectString(STR_ACCEPTED, 30000);
}

/**
* @brief Sends the command together with the given paramValue (optional)
* @param MAC param should include a trailing space if paramValue is set. Refer to RN2483 command ref.
* @param Param value to send
* @param Size of Param buffer
* @return Returns true on success or false if invalid.
*/
bool RN2483::setMacParam(const char* paramName, const uint8_t* paramValue, uint16_t size)
{
    _RN2483.printf(STR_CMD_SET);
    _RN2483.printf(paramName);

    for (uint16_t i = 0; i < size; ++i) {
        _RN2483.putc(static_cast<char>(NIBBLE_TO_HEX_CHAR(HIGH_NIBBLE(paramValue[i]))));
        _RN2483.putc(static_cast<char>(NIBBLE_TO_HEX_CHAR(LOW_NIBBLE(paramValue[i]))));
    }

    _RN2483.printf(CRLF);

    return expectOK();
}

/**
* @brief Sends the command together with the given paramValue (optional)
* @param MAC param should include a trailing space if paramValue is set. Refer to RN2483 command ref.
* @param Param value to send
* @return Returns true on success or false if invalid.
*/
bool RN2483::setMacParam(const char* paramName, uint8_t paramValue)
{
    _RN2483.printf(STR_CMD_SET);
    _RN2483.printf(paramName);
    _RN2483.printf("%u",paramValue);
    _RN2483.printf(CRLF);

    return expectOK();
}

/**
* @brief Sends the command together with the given paramValue (optional)
* @param MAC param should include a trailing space if paramValue is set. Refer to RN2483 command ref.
* @param Param value to send
* @return Returns true on success or false if invalid.
*/
bool RN2483::setMacParam(const char* paramName, const char* paramValue)
{
    _RN2483.printf(STR_CMD_SET);
    _RN2483.printf(paramName);
    _RN2483.printf(paramValue);
    _RN2483.printf(CRLF);

    return expectOK();
}

/**
* @brief Returns the enum that is mapped to the given "error" message
* @param Error to lookup.
* @return Returns the enum.
*/
uint8_t RN2483::lookupMacTransmitError(const char* error)
{
    if (error[0] == 0) {
        return NoResponse;
    }

    StringEnumPair_t errorTable[] = {
        { STR_RESULT_INVALID_PARAM, InternalError },
        { STR_RESULT_NOT_JOINED, NotConnected },
        { STR_RESULT_NO_FREE_CHANNEL, Busy },
        { STR_RESULT_SILENT, Silent },
        { STR_RESULT_FRAME_COUNTER_ERROR, NetworkFatalError },
        { STR_RESULT_BUSY, Busy },
        { STR_RESULT_MAC_PAUSED, InternalError },
        { STR_RESULT_INVALID_DATA_LEN, PayloadSizeError },
        { STR_RESULT_MAC_ERROR, NoAcknowledgment },
    };

    for (StringEnumPair_t * p = errorTable; p->stringValue != NULL; ++p) {
        if (strcmp(p->stringValue, error) == 0) {
            return p->enumValue;
        }
    }

    return NoResponse;
}

/**
* @brief Sends a a payload and blocks until there is a response back,
* or the receive windows have closed or the hard timeout has passed.
* @param Transmit type
* @param Port to use for transmit
* @param Payload buffer
* @param Size of payload buffer
* @return Returns if sucessfull or if a MAC transmit error.
*/
uint8_t RN2483::macTransmit(const char* type, uint8_t port, const uint8_t* payload, uint8_t size)
{
    _RN2483.printf(STR_CMD_MAC_TX);
    _RN2483.printf(type);
    _RN2483.printf("%u",port);
    _RN2483.printf(" ");

    for (int i = 0; i < size; ++i) {
        _RN2483.putc(static_cast<char>(NIBBLE_TO_HEX_CHAR(HIGH_NIBBLE(payload[i]))));
        _RN2483.putc(static_cast<char>(NIBBLE_TO_HEX_CHAR(LOW_NIBBLE(payload[i]))));
    }

    _RN2483.printf(CRLF);

    if (!expectOK()) {
        return lookupMacTransmitError(this->inputBuffer); // inputBuffer still has the last line read
    }

    this->packetReceived = false; // prepare for receiving a new packet
    Timer t;
    t.start();
    int timeout = t.read_ms() + RECEIVE_TIMEOUT; // hard timeouts
    while (t.read_ms() < timeout) {
        if (readLn() > 0) {
            if (strstr(this->inputBuffer, " ") != NULL) { // to avoid double delimiter search
                // there is a splittable line -only case known is mac_rx
                t.stop();
                return onMacRX();
            } else if (strstr(this->inputBuffer, STR_RESULT_MAC_TX_OK)) {
                // done
                t.stop();
                return NoError;
            } else {
                // lookup the error message
                t.stop();
                return lookupMacTransmitError(this->inputBuffer);
            }
        }
    }
    t.stop();
    return Timedout;
}

/**
* @brief Parses the input buffer and copies the received payload into
* the "received payload" buffer when a "mac rx" message has been received.
* @return Returns 0 (NoError) or otherwise one of the MacTransmitErrorCodes.
*/
uint8_t RN2483::onMacRX()
{
    // parse inputbuffer, put payload into packet buffer
    char* token = strtok(this->inputBuffer, " ");

    // sanity check
    if (strcmp(token, STR_RESULT_MAC_RX) != 0) {
        return InternalError;
    }

    // port
    token = strtok(NULL, " ");

    // payload
    token = strtok(NULL, " "); // until end of string

    uint16_t len = strlen(token) + 1; // include termination char
    memcpy(this->receivedPayloadBuffer, token, len <= this->receivedPayloadBufferSize ? len : this->receivedPayloadBufferSize);

    this->packetReceived = true; // enable receive() again
    return NoError;
}

/**
* @brief Private method to read serial port with timeout
* @param The time to wait for in milliseconds.
* @return Returns character or -1 on timeout
*/
int RN2483::timedRead(int _timeout)
{
    int c;
    Timer t;
    t.start();
    long _startMillis = t.read_ms(); // get milliseconds
    do { 
        if(_RN2483.readable()){
            c = _RN2483.getc();
            if (c >= 0){
                t.stop();
                return c;
            }
        }            
    } while(t.read_ms() - _startMillis <_timeout);
    t.stop();
    return -1; // -1 indicates timeout
}

/**
* @brief Read characters into buffer.
* Terminates if length characters have been read, timeout, or
* if the terminator character has been detected
* @param The terminator character to look for
* @param The buffer to read into.
* @param The number of bytes to read.
* @return The number of bytes read. 0 means no valid data found.
*/
size_t RN2483::readBytesUntil(char terminator, char *buffer, size_t bytesToRead)
{
    if (bytesToRead < 1) return 0;
    size_t index = 0;
    while (index < (bytesToRead - 1 )) {
        int c = timedRead(1000);
        if (c < 0 || c == terminator) break;
        *buffer++ = (char)c;
        index++;
    }
    return index; // return number of characters, not including null terminator
}

#ifdef DEBUG
int freeRam()
{
    extern int __heap_start;
    extern int *__brkval;
    int v;
    return (int)&v - (__brkval == 0 ? (int)&__heap_start : (int)__brkval);
}
#endif
