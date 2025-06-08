//! Module to handle button short and long presses and sends signals to the player.

use embassy_rp::gpio::{AnyPin, Input};

use crate::player::PlayerCmdSender;

// something like this:
//   wait for pressed
//   select(wait for released, 1s timeout)
//     if it's released first, it's a short press
//     if the timeout elapses first, it's a long press"

#[embassy_executor::task]
pub async fn button_task(
    btn_previous: Input<'static>, btn_play: Input<'static>, btn_next: Input<'static>, sender: PlayerCmdSender) {
    todo!();
}
