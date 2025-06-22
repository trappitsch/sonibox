//! RFID reader module
//!
//! Currently, the library does not support the interrupt pin at all.
//! We'll deal with some basic timer and reset a timer functionality to disable the loop or enable
//! it (TODO).

use defmt::{error, info};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_futures::select::{Either, select};
use embassy_rp::{gpio::Output, spi};
use embassy_time::{Duration, Timer};
use mfrc522::{Mfrc522, comm::blocking::spi::SpiInterface};

use crate::{
    SpiBusType,
    leds::{LED_CMD_CHANNEL, LedCommand},
    player::{PlayerCmdSender, PlayerCommand},
    sleep::{AWAKE_SIGNAL, AwakeCmd, DEV_STAT_SIGNAL, DeviceStatus},
    tags::folder_selection,
};

const POLLING_INTERVAL: Duration = Duration::from_millis(500);

#[embassy_executor::task]
pub async fn rfid_task(spi_bus: SpiBusType, touch_cs: Output<'static>, sender: PlayerCmdSender) {
    let spi_device = SpiDeviceWithConfig::new(&spi_bus, touch_cs, spi::Config::default());
    let itf = SpiInterface::new(spi_device);
    let mut rfid = Mfrc522::new(itf).init().unwrap();

    // Unfortunately, this library does not include async nor interrupt support.
    info!("Starging RFID reader loop...");
    let mut current_folder: Option<usize> = None;
    loop {
        let which_one = select(Timer::after(POLLING_INTERVAL), DEV_STAT_SIGNAL.wait()).await;
        match which_one {
            Either::First(_) => {
                if let Ok(atqa) = rfid.reqa() {
                    while let Ok(uid) = rfid.select(&atqa) {
                        let folder_opt = folder_selection(uid.as_bytes());
                        if current_folder != folder_opt {
                            if let Some(folder) = folder_opt {
                                info!("Selected folder: {}", folder);
                                AWAKE_SIGNAL.signal(AwakeCmd::StayAwake);
                                sender.send(PlayerCommand::PlayFolder(folder as u8)).await;
                            }
                            current_folder = folder_opt;
                        }
                        Timer::after(POLLING_INTERVAL).await;
                    }
                } else if current_folder.is_some() {
                    info!("Card removed. Stop playing.");
                    AWAKE_SIGNAL.signal(AwakeCmd::StayAwake);
                    sender.send(PlayerCommand::Stop).await;
                    current_folder = None;
                }
            }
            Either::Second(_) => {
                info!("Stop polling the RFID reader.");
                let sig = DEV_STAT_SIGNAL.wait().await;
                if sig == DeviceStatus::Sleeping {
                    error!(
                        "RFID loop got a go to sleep signal while sleeping: This should not happen."
                    );
                    DEV_STAT_SIGNAL.signal(DeviceStatus::Sleeping);
                    LED_CMD_CHANNEL.send(LedCommand::Error).await;
                }
                // resuming loop
            }
        }
    }
}
