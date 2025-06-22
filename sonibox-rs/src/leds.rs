//! This handles the LEDs

use embassy_rp::gpio::Output;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::Channel,
};
use embassy_time::{Duration, Timer};

const DIT_MS: u64 = 250;
const DIT: Duration = Duration::from_millis(DIT_MS);
const DAH: Duration = Duration::from_millis(3 * DIT_MS);

#[derive(Debug)]
pub enum LedCommand {
    Error,      // SOS pattern on all LEDs
    Off,
}

pub type LedCmdChannel = Channel<ThreadModeRawMutex, LedCommand, 16>;
pub static LED_CMD_CHANNEL: LedCmdChannel = Channel::new();

pub struct Leds<'a> {
    blue: Output<'a>,
    green: Output<'a>,
    yellow: Output<'a>,
}

impl<'a> Leds<'a> {
    pub fn new(blue: Output<'a>, green: Output<'a>, yellow: Output<'a>) -> Self {
        Leds {
            blue,
            green,
            yellow,
        }
    }

    fn all_on(&mut self) {
        self.blue.set_high();
        self.green.set_high();
        self.yellow.set_high();
    }

    fn all_off(&mut self) {
        self.blue.set_low();
        self.green.set_low();
        self.yellow.set_low();
    }

    /// Signal SOS once around, with proper end timing.
    async fn sos_once(&mut self) {
        for _ in 0..3 {
            self.all_on();
            Timer::after(DIT).await;
            self.all_off();
            Timer::after(DIT).await;
        }
        Timer::after(2*DIT).await;
        for _ in 0..3 {
            self.all_on();
            Timer::after(DAH).await;
            self.all_off();
            Timer::after(DIT).await;
        }
        Timer::after(2*DIT).await;
        for _ in 0..3 {
            self.all_on();
            Timer::after(DIT).await;
            self.all_off();
            Timer::after(DIT).await;
        }
        Timer::after(2*DIT).await;
    }
}

#[embassy_executor::task]
pub async fn led_task(mut leds: Leds<'static>) {
    loop {
        match LED_CMD_CHANNEL.receive().await {
            LedCommand::Off => leds.all_off(),
            LedCommand::Error => {
                // doesn't break until reboot!
                loop {
                    leds.sos_once().await;
                }
            }
        }
    }
}
