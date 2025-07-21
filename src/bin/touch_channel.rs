#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};

use embassy_stm32::mode::Blocking;
use embassy_stm32::{i2c::I2c, time::Hertz};

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Timer};
use embedded_graphics::geometry::Point;

use ft5336;
//Declate a channel of 2 u32s
static SHARED: Channel<ThreadModeRawMutex, Point, 1> = Channel::new();
static mut DELAY: Delay = Delay;

use {defmt_rtt as _, panic_probe as _};

// Basic Blink Task
#[embassy_executor::task]
async fn blink(mut led: Output<'static>) {
    loop {
        info!("LED on");
        led.set_high();
        Timer::after_millis(300).await;

        info!("LED off");
        led.set_low();
        Timer::after_millis(300).await;
    }
}

#[embassy_executor::task]
async fn catch_touch(
    mut touch: ft5336::Ft5336<'static, I2c<'static, Blocking>>,
    mut i2c: I2c<'static, Blocking>,
) {
    loop {
        let t = touch.detect_touch(&mut i2c);
        let mut num: u8 = 0;
        match t {
            Err(e) => error!("Error {} from fetching number of touches", e),
            Ok(n) => {
                num = n;
                if num != 0 {
                    info!("Number of touches: {}", num)
                };
            }
        }
        if num > 0 {
            let t = touch.get_touch(&mut i2c, 1);
            match t {
                Err(_e) => error!("Error fetching touch data"),
                Ok(n) => {
                    let p = Point::new(n.x.into(), n.y.into());
                    info!(
                        "Touch: {}x{} - weight: {} misc: {}",
                        p.y, p.x, n.weight, n.misc
                    );
                    // Send to Channel touch via Point
                    SHARED.send(p).await;
                }
            }
        }

        Timer::after_millis(50).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Touch Channel Example!");

    let led = Output::new(p.PI1, Level::Low, Speed::Low);

    let i2c = I2c::new_blocking(p.I2C3, p.PH7, p.PH8, Hertz(50_000), Default::default());

    let delay_ref: &'static mut Delay = unsafe { &mut DELAY };
    let touch = ft5336::Ft5336::new(&i2c, 0x38, delay_ref).unwrap();

    spawner.spawn(blink(led)).unwrap();
    spawner.spawn(catch_touch(touch, i2c)).unwrap();

    loop {
        let point = SHARED.receive().await;
        info!("Channel Received data: {}x{}", point.y, point.x);
    }
}
