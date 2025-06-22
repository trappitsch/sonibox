//! This module defines a channel to put the device into sleep mode.
//!
//! Since the mfrc522 driver does not incorporate async and interrupt driven card detection, this
//! module is implemented to provide a driver. After a defined amount of idle time, we will stop
//! the loop that polls for RFID cards. Then, only button presses (which are async), will wake up
//! the box again.

use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};

use crate::player::{PLAYER_CMD_CHANNEL, PLAYER_STAT_CHANNEL, PlayerCommand};

const IDLE_TIME: Duration = Duration::from_secs(300);  // 5 min

pub enum AwakeCmd {
    StayAwake,
}

#[derive(Debug, PartialEq)]
pub enum DeviceStatus {
    Sleeping,
    Awake,
}

pub static AWAKE_SIGNAL: Signal<CriticalSectionRawMutex, AwakeCmd> = Signal::new();
pub static DEV_STAT_SIGNAL: Signal<CriticalSectionRawMutex, DeviceStatus> = Signal::new();

pub struct SleepTimer {
    last_activity: Instant,
}

impl SleepTimer {
    pub fn new() -> Self {
        SleepTimer {
            last_activity: Instant::now(),
        }
    }

    fn reset(&mut self) {
        self.last_activity = Instant::now();
    }

    pub async fn wait(&mut self) {
        loop {
            let dur_since_last_activity = Instant::now() - self.last_activity;

            // are we done and going to sleep now?
            if dur_since_last_activity >= IDLE_TIME {
                PLAYER_CMD_CHANNEL.send(PlayerCommand::GetStatus).await;
                let current_status = PLAYER_STAT_CHANNEL.receive().await;

                if current_status.is_stopped() {
                    info!("Going to sleep...");
                    DEV_STAT_SIGNAL.signal(DeviceStatus::Sleeping);
                    let _ = AWAKE_SIGNAL.wait().await; // wait for a wake up signal
                    info!("Waking up...");
                    DEV_STAT_SIGNAL.signal(DeviceStatus::Awake);
                    self.reset();
                } else {
                    info!("Can't sleep cause I'm playing");
                    self.reset();
                }
            } else {
                let time_to_next_check = IDLE_TIME - dur_since_last_activity;
                let which_ended =
                    select(AWAKE_SIGNAL.wait(), Timer::after(time_to_next_check)).await;
                match which_ended {
                    Either::First(_) => {
                        self.reset();
                    }
                    Either::Second(_) => (),
                }
            }
        }
    }
}
