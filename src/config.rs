//! # Device Configuration
//!
//! The device configuration is read from EEPROM.
//!
//! ## Memory Map
//!
//!                 0           8          16          24          32
//!                 +-----------+-----------+-----------+-----------+
//!     0x0808_0000 | Version   | Reserved                          |
//!                 +-----------+-----------+-----------+-----------+
//!     0x0808_0004 | DevAddr                                       |
//!                 +-----------+-----------+-----------+-----------+
//!     0x0808_0008 |                                               |
//!     0x0808_000C | NwkSKey                                       |
//!     0x0808_0010 |                                               |
//!     0x0808_0014 |                                               |
//!                 +-----------+-----------+-----------+-----------+
//!     0x0808_0018 |                                               |
//!     0x0808_001C | AppSKey                                       |
//!     0x0808_0020 |                                               |
//!     0x0808_0024 |                                               |
//!                 +-----------+-----------+-----------+-----------+
//!     0x0808_0028 | WakeupInterval        | ITempHumi | IVoltage  |
//!                 +-----------+-----------+-----------+-----------+
//!
//! ## Fields
//!
//! ### Header (0x0808_0000 - 0x0808_0004, 4 bytes)
//!
//! - `Version`: The constant `0x01`, can be used to change the config layout
//!   in the future (1 byte)
//! - The other three bytes are reserved
//!
//! ### LoRaWAN Configuration (0x0808_0004 - 0x0808_0028, 36 bytes)
//!
//! - `DevAddr`: LoRaWAN device address (4 bytes)
//! - `NwkSKey`: LoRaWAN ABP network session key (16 bytes)
//! - `AppSKey`: LoRaWAN ABP app session key (16 bytes)
//!
//! ### Interval Configuration (0x0808_0028 - 0x0808_002C, 4 bytes)
//!
//! - `WakeupInterval`: How often (in seconds) the device should wake up to
//!   start measurement(s) (2 bytes, u16, LE)
//! - `ITempHumi`: Every n-th measurement will measure and send temperature and
//!   humidity (1 byte, u8)
//! - `IVoltage`: Every n-th measurement will measure and send battery
//!   voltage (1 byte, u8)
//!
//! Example: With the following value at `0x0808_0028`:
//!
//!     +-------------------------------------------+
//!     | 00000384   00000000 | 00000001 | 00000004 |
//!     +-------------------------------------------+
//!
//! ...the temperature and humidity will be sent every 15 minutes, while the
//! voltage will be sent every hour.
