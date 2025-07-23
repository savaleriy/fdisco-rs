#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::init;
use embassy_time::{Duration, Timer};
use panic_probe as _;

// -------- HAL: LED GPIO --------
struct Led {
    pin: Output<'static>,
}

impl Led {
    fn new(pin: embassy_stm32::peripherals::PI1) -> Self {
        Self {
            pin: Output::new(pin, Level::Low, Speed::Low),
        }
    }

    fn toggle(&mut self) {
        self.pin.toggle();
    }
}

// -------- HAL: Timer abstraction (like SysTick every 1ms) --------
struct SysTimer {
    tick_ms: u32,
}

impl SysTimer {
    fn new() -> Self {
        Self { tick_ms: 0 }
    }

    async fn run<F: FnMut()>(&mut self, mut on_500ms: F) -> ! {
        loop {
            Timer::after(Duration::from_millis(1)).await;
            self.tick_ms += 1;

            if self.tick_ms >= 500 {
                on_500ms();
                self.tick_ms = 0;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = init(Default::default());
    info!("SysTick example");

    let mut led = Led::new(p.PI1);
    let mut timer = SysTimer::new();

    info!("Starting SysTick-like timer with LED toggle every 500ms");

    // Run the timer loop, toggling the LED every 500ms
    timer
        .run(|| {
            info!("Toggling LED");
            led.toggle();
        })
        .await;
}
