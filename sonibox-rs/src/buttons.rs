//! Module to handle button short and long presses and sends signals to the player.

use defmt::{Debug2Format, info};
use embassy_futures::select::{select, select3};
use embassy_rp::gpio::{Input, Level};
use embassy_time::{Duration, Timer};

use crate::{player::{PlayerCmdSender, PlayerCommand, PLAYER_CMD_CHANNEL}, sleep::{AwakeCmd, AWAKE_SIGNAL}};

const DEBOUNCE_DURATION: Duration = Duration::from_millis(200);
const SECONDARY_ACTION_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug)]
enum ButtonType {
    Next,      // Next track (short), volume oup (long press)
    Previous,  // Previous track (short), volume down (long press)
    PlayPause, // Play or pause (short), nothing (long press)
}

struct DualUseButton<'a> {
    input: Input<'a>,
    sender: PlayerCmdSender,
    button_type: ButtonType,
}

impl<'a> DualUseButton<'a> {
    pub fn new(input: Input<'a>, button_type: ButtonType) -> Self {
        Self {
            input,
            sender: PLAYER_CMD_CHANNEL.sender(),
            button_type,
        }
    }

    pub async fn action(&mut self) {
        self.input.wait_for_low().await; // button was pressed
        let mut is_secondary_action = false;

        Timer::after(DEBOUNCE_DURATION).await;
        let l2 = self.input.get_level();
        if l2 == Level::High {
            info!(
                "Button {:?} pressed and released quickly",
                Debug2Format(&self.button_type)
            );
            match self.button_type {
                ButtonType::Next => self.sender.send(PlayerCommand::Next).await,
                ButtonType::Previous => self.sender.send(PlayerCommand::Previous).await,
                ButtonType::PlayPause => self.sender.send(PlayerCommand::PlayPause).await,
            }
            return;
        }

        // button still pressed, wait for release or timeout for secondary action
        match self.button_type {
            ButtonType::PlayPause => {
                self.input.wait_for_high().await;
                self.sender.send(PlayerCommand::PlayPause).await;
                info!("Play button released");
            }
            ButtonType::Next => loop {
                select(
                    self.input.wait_for_high(),
                    Timer::after(SECONDARY_ACTION_TIMEOUT),
                )
                .await;
                if self.input.get_level() == Level::High {
                    if !is_secondary_action {
                        info!(
                            "Next button released after debounce, primary action, sending Next command"
                        );
                        self.sender.send(PlayerCommand::Next).await;
                    };
                    break;
                } else {
                    info!("Next button long pressed, sending Volume Up command");
                    is_secondary_action = true;
                    self.sender.send(PlayerCommand::VolumeUp).await;
                }
            },
            ButtonType::Previous => loop {
                select(
                    self.input.wait_for_high(),
                    Timer::after(SECONDARY_ACTION_TIMEOUT),
                )
                .await;
                if self.input.get_level() == Level::High {
                    if !is_secondary_action {
                        info!(
                            "Previous button released after debounce, primary action, sending Previous command"
                        );
                        self.sender.send(PlayerCommand::Previous).await;
                    };
                    break;
                } else {
                    info!("Previous button long pressed, sending Volume Down command");
                    is_secondary_action = true;
                    self.sender.send(PlayerCommand::VolumeDown).await;
                }
            },
        }
        Timer::after(DEBOUNCE_DURATION).await; // wait for debounce time after release
    }
}

#[embassy_executor::task]
pub async fn button_task(
    btn_previous_inp: Input<'static>,
    btn_play_inp: Input<'static>,
    btn_next_inp: Input<'static>,
) {
    let mut btn_previous = DualUseButton::new(btn_previous_inp, ButtonType::Previous);
    let mut btn_play = DualUseButton::new(btn_play_inp, ButtonType::PlayPause);
    let mut btn_next = DualUseButton::new(btn_next_inp, ButtonType::Next);
    info!("Starting button task loop...");
    loop {
        select3(btn_previous.action(), btn_play.action(), btn_next.action()).await;
        AWAKE_SIGNAL.signal(AwakeCmd::StayAwake);
    }
}
