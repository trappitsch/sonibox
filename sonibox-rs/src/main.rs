#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    config::Config,
    gpio::{Input, Pull},
    peripherals::UART0,
    uart::{
        BufferedInterruptHandler, BufferedUart, Config as UartConfig, DataBits, Parity, StopBits,
    },
};

use defmt_rtt as _;
use panic_probe as _;
use static_cell::StaticCell;

use buttons::button_task;
use player::{
    player_task,
    PLAYER_CMD_CHANNEL,
};

mod buttons;
mod player;

bind_interrupts!(pub struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

static TX_BUF: StaticCell<[u8; 128]> = StaticCell::new();
static RX_BUF: StaticCell<[u8; 128]> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    
    // set up buttons
    let btn_previous = Input::new(p.PIN_18, Pull::Up);
    let btn_play= Input::new(p.PIN_19, Pull::Up);
    let btn_next= Input::new(p.PIN_20, Pull::Up);

    // set up UART for the DFPlayer
    let mut uart_config = UartConfig::default();
    uart_config.baudrate = 9600;
    uart_config.data_bits = DataBits::DataBits8;
    uart_config.stop_bits = StopBits::STOP1;
    uart_config.parity = Parity::ParityNone;

    let tx_buf = &mut TX_BUF.init([0; 128])[..];
    let rx_buf = &mut RX_BUF.init([0; 128])[..];
    let uart = BufferedUart::new(
        p.UART0,
        Irqs,
        p.PIN_16,
        p.PIN_17,
        tx_buf,
        rx_buf,
        uart_config,
    );

    spawner.must_spawn(player_task(uart, PLAYER_CMD_CHANNEL.receiver()));
    spawner.must_spawn(button_task(btn_previous, btn_play, btn_next, PLAYER_CMD_CHANNEL.sender()));

    loop {
        // Main loop can handle other tasks or just sleep
        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
        PLAYER_CMD_CHANNEL.sender().send(player::PlayerCommand::Play).await;
    }
}
