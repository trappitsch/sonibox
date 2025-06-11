//! RFID reader module
//!
//! Currently, the library does not support the interrupt pin at all.
//! We'll deal with some basic timer and reset a timer functionality to disable the loop or enable
//! it (TODO).

use defmt::{Debug2Format, info, warn};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{gpio::Output, spi};
use embassy_time::{Duration, Timer};
use mfrc522::{Mfrc522, comm::blocking::spi::SpiInterface};

use crate::{player::PlayerCmdSender, SpiBusType};

const POLLING_INTERVAL: Duration = Duration::from_millis(500);

#[embassy_executor::task]
pub async fn rfid_task(spi_bus: SpiBusType, touch_cs: Output<'static>, sender: PlayerCmdSender) {
    let spi_device = SpiDeviceWithConfig::new(&spi_bus, touch_cs, spi::Config::default());
    let itf = SpiInterface::new(spi_device);
    let mut rfid = Mfrc522::new(itf).init().unwrap();

    // Unfortunately, this library does not include async nor interrupt support.
    loop {
        Timer::after(POLLING_INTERVAL).await;
        if let Ok(atqa) = rfid.reqa() {
            if let Ok(uid) = rfid.select(&atqa) {
                info!("UID: {:?}", Debug2Format(&uid.as_bytes()));
            } else {
                warn!("Failed to select UID");
            }
        }
    }
}
