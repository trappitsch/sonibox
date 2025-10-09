//! Player module for handling playback-related functionality.

use defmt::{Debug2Format, error, info};
use dfplayer_async::{DfPlayer, TimeSource};
use embassy_rp::{peripherals as p, uart::BufferedUart};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::{Channel, Sender},
};
use embassy_time::{Delay, Duration, Instant, Timer};

use crate::leds::{LED_CMD_CHANNEL, LedCommand};

pub type PlayerCmdChannel = Channel<ThreadModeRawMutex, PlayerCommand, 16>;
pub type PlayerCmdSender = Sender<'static, ThreadModeRawMutex, PlayerCommand, 16>;
pub static PLAYER_CMD_CHANNEL: PlayerCmdChannel = Channel::new();

// A channel that will send the player status. To request it, send a playercommand
pub static PLAYER_STAT_CHANNEL: Channel<ThreadModeRawMutex, PlayerStatus, 16> = Channel::new();

#[derive(Debug)]
pub enum PlayerCommand {
    PlayFolder(u8), // Play a specific folder
    PlayPause,      // Just play after a pause
    Stop,           // Stop playback
    Next,           // Play the next track
    Previous,       // Play the previous track
    VolumeUp,       // Increase volume
    VolumeDown,     // Decrease volume
    GetStatus,      // Get the player to emit a PLAYER_STAT_CHANNEL signal
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PlayerStatus {
    Playing, // can be paused (button) or stopped (remove figure)
    Paused,  // playback is paused but figure is still present
    Stopped, // playback is stopped because there is no figure
}

/// Volume structure. Minimum is 0, maximum is 30.
struct Volume {
    current: u8,
}

impl Volume {
    fn new() -> Self {
        Self { current: 15 }
    }

    fn increase_volume(&mut self) {
        if self.current < 30 {
            self.current += 1;
        }
    }

    fn decrease_volume(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }
}

impl PlayerStatus {
    pub fn is_playing(&self) -> bool {
        self == &PlayerStatus::Playing
    }

    pub fn is_stopped(&self) -> bool {
        self == &PlayerStatus::Stopped
    }

    fn pause(&mut self) {
        *self = PlayerStatus::Paused;
    }

    fn play(&mut self) {
        *self = PlayerStatus::Playing;
    }

    fn stop(&mut self) {
        *self = PlayerStatus::Stopped;
    }
}

/// The required time source for the player module, implemented as in the example.
struct MyTimeSource;

impl TimeSource for MyTimeSource {
    type Instant = Instant;

    fn now(&self) -> Self::Instant {
        Instant::now()
    }

    fn is_elapsed(&self, since: Self::Instant, timeout_ms: u64) -> bool {
        Instant::now().duration_since(since) >= Duration::from_millis(timeout_ms)
    }
}

#[embassy_executor::task]
pub async fn player_task(mut uart: BufferedUart<'static, p::UART0>) {
    // configuration for DFPlayer
    let feedback_enable = true;
    let timeout_ms = 1000;
    let delay = Delay;
    let reset_during_override = None;

    let mut volume = Volume::new();

    let mut player_status = PlayerStatus::Stopped;

    let mut last_played: Option<u8> = None; // last folder played

    info!("Initializing DFPlayer Mini...");
    let mut dfplayer = match DfPlayer::new(
        &mut uart,
        feedback_enable,
        timeout_ms,
        MyTimeSource,
        delay,
        reset_during_override,
    )
    .await
    {
        Ok(dfplayer) => dfplayer,
        Err(e) => {
            error!(
                "Failed to initialize DFPlayer Mini: {:?}.",
                Debug2Format(&e)
            );
            LED_CMD_CHANNEL.send(LedCommand::Error).await;
            return;
        }
    };
    info!("DFPlayer Mini initialized successfully.");

    Timer::after(Duration::from_millis(100)).await; // give some time to the player to settle

    // Set up volume to intialize the player with
    match dfplayer.set_volume(volume.current).await {
        Ok(_) => info!("Standard volume set: {}", volume.current),
        Err(e) => {
            error!("Failed to set volume: {:?}", Debug2Format(&e));
        }
    }

    // loop and wait for command from other places...
    info!("Entering player loop...");
    loop {
        match PLAYER_CMD_CHANNEL.receive().await {
            PlayerCommand::PlayPause => match player_status {
                PlayerStatus::Paused => {
                    LED_CMD_CHANNEL.send(LedCommand::AllOn).await;
                    player_status.play();
                    match dfplayer.resume().await {
                        Ok(_) => info!("Resumed playback."),
                        Err(e) => error!("Failed to resume playback: {:?}", Debug2Format(&e)),
                    }
                }
                PlayerStatus::Playing => {
                    LED_CMD_CHANNEL.send(LedCommand::OnlyPlay).await;
                    player_status.pause();
                    match dfplayer.pause().await {
                        Ok(_) => info!("Paused playback."),
                        Err(e) => error!("Failed to pause playback: {:?}", Debug2Format(&e)),
                    }
                }
                PlayerStatus::Stopped => {
                    info!("Player is stopped, cannot play/pause.");
                    LED_CMD_CHANNEL.send(LedCommand::InvalidCommand).await;
                }
            },
            PlayerCommand::Stop => {
                player_status.stop();
                LED_CMD_CHANNEL.send(LedCommand::Off).await;
                // status is stopped, but we only pause. If same figure comes again, continue
                match dfplayer.pause().await {
                    Ok(_) => info!("Paused playback."),
                    Err(e) => error!("Failed to stop playback: {:?}", Debug2Format(&e)),
                }
            }
            PlayerCommand::Next => {
                if player_status.is_playing() {
                    match dfplayer.next().await {
                        Ok(_) => info!("Playing next track."),
                        Err(e) => error!("Failed to play next track: {:?}", Debug2Format(&e)),
                    }
                }
            }
            PlayerCommand::Previous => {
                if player_status.is_playing() {
                    match dfplayer.previous().await {
                        Ok(_) => info!("Playing previous track."),
                        Err(e) => error!("Failed to play previous track: {:?}", Debug2Format(&e)),
                    }
                }
            }
            PlayerCommand::PlayFolder(folder) => {
                LED_CMD_CHANNEL.send(LedCommand::AllOn).await;
                player_status.play();
                if last_played == Some(folder) {
                    // same figure again!
                    info!("Continue playing with same figure, folder {}.", folder);
                    match dfplayer.resume().await {
                        Ok(_) => info!("Resumed playback."),
                        Err(e) => error!("Failed to resume playback: {:?}", Debug2Format(&e)),
                    }
                } else {
                    info! {"Playing folder {}.", folder};
                    last_played = Some(folder);
                    match dfplayer.play_loop_folder(folder).await {
                        Ok(_) => info!("Playing folder {}.", folder),
                        Err(e) => {
                            error!("Failed to play folder {}: {:?}.", folder, Debug2Format(&e))
                        }
                    }
                }
            }
            PlayerCommand::VolumeUp => {
                if player_status.is_playing() {
                    volume.increase_volume();
                    match dfplayer.set_volume(volume.current).await {
                        Ok(_) => info!("Volume increased to: {}", volume.current),
                        Err(e) => {
                            error!("Failed to set volume: {:?}", Debug2Format(&e));
                        }
                    }
                } else {
                    info!("Player is not playing, volume increase ignored.");
                }
            }
            PlayerCommand::VolumeDown => {
                if player_status.is_playing() {
                    volume.decrease_volume();
                    match dfplayer.set_volume(volume.current).await {
                        Ok(_) => info!("Volume decreased to: {}", volume.current),
                        Err(e) => {
                            error!("Failed to set volume: {:?}", Debug2Format(&e));
                        }
                    }
                } else {
                    info!("Player is not playing, volume decrease ignored.");
                }
            }
            PlayerCommand::GetStatus => {
                PLAYER_STAT_CHANNEL.send(player_status).await;
            }
        }
    }
}
