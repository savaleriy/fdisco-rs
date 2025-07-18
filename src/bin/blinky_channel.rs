#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::{Duration, Ticker};
use gpio::{AnyPin, Level, Output, Speed};
use {defmt_rtt as _, panic_probe as _};

enum LedState {
    Toggle,
}
static CHANNEL: Channel<ThreadModeRawMutex, LedState, 64> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Blinky Channel example");

    let mut led = Output::new(AnyPin::from(p.PI1), Level::High, Speed::Low);

    let dt = 100 * 1_000_000;
    let k = 1.003;

    unwrap!(spawner.spawn(toggle_led(CHANNEL.sender(), Duration::from_nanos(dt))));
    unwrap!(spawner.spawn(toggle_led(
        CHANNEL.sender(),
        Duration::from_nanos((dt as f64 * k) as u64)
    )));

    loop {
        match CHANNEL.receive().await {
            LedState::Toggle => led.toggle(),
        }
    }
}

// A pool size of 2 means you can spawn two instances of this task.
#[embassy_executor::task(pool_size = 2)]
async fn toggle_led(control: Sender<'static, ThreadModeRawMutex, LedState, 64>, delay: Duration) {
    let mut ticker = Ticker::every(delay);
    loop {
        control.send(LedState::Toggle).await;
        info!("Toggle");
        ticker.next().await;
    }
}
