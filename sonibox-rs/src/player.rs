//! Player module for handling playback-related functionality.

use defmt::{Debug2Format, error, info};
use dfplayer_async::{Command, DfPlayer, MessageData, PlayBackMode, TimeSource};
use embassy_rp::{peripherals as p, uart::BufferedUart};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Delay, Duration, Instant, Timer};

pub type PlayerCmdChannel = Channel<ThreadModeRawMutex, PlayerCommand, 16>;
pub type PlayerCmdSender = Sender<'static, ThreadModeRawMutex, PlayerCommand, 16>;
pub type PlayerCmdReceiver = Receiver<'static, ThreadModeRawMutex, PlayerCommand, 16>;
pub static PLAYER_CMD_CHANNEL: PlayerCmdChannel = Channel::new();

pub enum PlayerCommand {
    PlayFolder(u8), // Play a specific folder
    PlayPause,      // Just play after a pause
    Stop,           // Stop playback
    Next,           // Play the next track
    Previous,       // Play the previous track
    VolumeUp,       // Increase volume
    VolumeDown,     // Decrease volume
}

#[derive(Debug, PartialEq)]
enum PlayerStatus {
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
        Self { current: 7 }
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
    fn is_paused(&self) -> bool {
        self == &PlayerStatus::Paused
    }

    fn is_playing(&self) -> bool {
        self == &PlayerStatus::Playing
    }

    fn is_stopped(&self) -> bool {
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
pub async fn player_task(mut uart: BufferedUart<'static, p::UART0>, receiver: PlayerCmdReceiver) {
    // configuration for DFPlayer
    let feedback_enable = false;
    let timeout_ms = 1000;
    let delay = Delay;
    let reset_during_override = None;

    let mut volume = Volume::new();

    let mut player_status = PlayerStatus::Stopped;

    let mut dfplayer = match DfPlayer::try_new(
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
            // FIXME: This needs to be handled differently for prod.
            error!(
                "Failed to initialize DFPlayer Mini: {:?}.",
                Debug2Format(&e)
            );
            return;
        }
    };

    Timer::after(Duration::from_millis(100)).await; // give some time to the player to settle

    // set the player into loop tracks in folder mode
    // HACK: This is a total workaround and should be implemented in the library properly...
    let cmd = Command::PlayLoopTrack;
    let message_data = MessageData::new(cmd, 0, PlayBackMode::FolderRepeat as u8);
    match dfplayer.send_command(message_data).await {
        Ok(_) => info!("Set player to loop tracks in folder mode."),
        Err(e) => {
            error!(
                "Failed to set player to loop tracks: {:?}",
                Debug2Format(&e)
            );
            return;
        }
    }

    // Set up volume to intialize the player with
    match dfplayer.set_volume(volume.current).await {
        Ok(_) => info!("Standard volume set: {}", volume.current),
        Err(e) => {
            error!("Failed to set volume: {:?}", Debug2Format(&e));
            return;
        }
    }

    // loop and wait for command from other places...
    loop {
        match receiver.receive().await {
            PlayerCommand::PlayPause => match player_status {
                PlayerStatus::Paused => {
                    player_status.play();
                    match dfplayer.resume().await {
                        Ok(_) => info!("Resumed playback."),
                        Err(e) => error!("Failed to resume playback: {:?}", Debug2Format(&e)),
                    }
                }
                PlayerStatus::Playing => {
                    player_status.pause();
                    match dfplayer.pause().await {
                        Ok(_) => info!("Paused playback."),
                        Err(e) => error!("Failed to pause playback: {:?}", Debug2Format(&e)),
                    }
                }
                PlayerStatus::Stopped => {
                    info!("Player is stopped, cannot play/pause.");
                }
            },
            PlayerCommand::Stop => {
                player_status.stop();
                match dfplayer.stop().await {
                    Ok(_) => info!("Stopped playback."),
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
                player_status.play();
                match dfplayer.play_from_folder(folder, 1).await {
                    Ok(_) => info!("Playing folder {}.", folder),
                    Err(e) => error!("Failed to play folder {}: {:?}", folder, Debug2Format(&e)),
                }
            }
            PlayerCommand::VolumeUp => {
                if player_status.is_playing() {
                    volume.increase_volume();
                    match dfplayer.set_volume(volume.current).await {
                        Ok(_) => info!("Volume increased to: {}", volume.current),
                        Err(e) => {
                            error!("Failed to set volume: {:?}", Debug2Format(&e));
                            return;
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
                            return;
                        }
                    }
                } else {
                    info!("Player is not playing, volume decrease ignored.");
                }
            }
        }
    }
}
