#![no_std]
#![no_main]

use core::cell::RefCell;

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    config::Config,
    gpio::{Input, Level, Output, Pull},
    peripherals::{self, UART0},
    spi::{self, Spi},
    uart::{
        BufferedInterruptHandler, BufferedUart, Config as UartConfig, DataBits, Parity, StopBits,
    },
};
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};

use defmt_rtt as _;
use panic_probe as _;
use static_cell::StaticCell;

use buttons::button_task;
use player::{PLAYER_CMD_CHANNEL, player_task};
use rfid::rfid_task;

mod buttons;
mod player;
mod rfid;
mod tags;

bind_interrupts!(pub struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

type SpiBusType = Mutex<NoopRawMutex, RefCell<spi::Spi<'static, peripherals::SPI0, spi::Blocking>>>;

static TX_BUF: StaticCell<[u8; 128]> = StaticCell::new();
static RX_BUF: StaticCell<[u8; 128]> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Config::default());

    // set up buttons
    let btn_previous = Input::new(p.PIN_18, Pull::Up);
    let btn_play = Input::new(p.PIN_19, Pull::Up);
    let btn_next = Input::new(p.PIN_20, Pull::Up);

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

    // set up spi for the rfid reader
    let cipo = p.PIN_4;
    let copi = p.PIN_3;
    let clk = p.PIN_2;
    let touch_cs = p.PIN_5;
    let touch_cs_out = Output::new(touch_cs, Level::High);

    // create SPI for RFID reader
    let spi_blocking = Spi::new_blocking(p.SPI0, clk, copi, cipo, spi::Config::default());
    let spi_bus: SpiBusType = Mutex::new(RefCell::new(spi_blocking));

    spawner.must_spawn(player_task(uart, PLAYER_CMD_CHANNEL.receiver()));
    spawner.must_spawn(button_task(
        btn_previous,
        btn_play,
        btn_next,
        PLAYER_CMD_CHANNEL.sender(),
    ));
    spawner.must_spawn(rfid_task(spi_bus, touch_cs_out, PLAYER_CMD_CHANNEL.sender()));

    loop {
        // Main loop can handle other tasks or just sleep
        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
    }
}

