#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    i2c::{Error, I2c},
    time::{Hertz},
};
use embassy_time::{Instant, Timer, Delay, Duration};
use {defmt_rtt as _, panic_probe as _};


extern crate ft5336;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Starting I2C Scanner with Diagnostic Logging");

    // LED for visual feedback
    let mut led = Output::new(p.PI1, Level::High, Speed::Low);

    // I2C Configuration - slower speed for better reliability during scanning
    let mut i2c = I2c::new_blocking(
        p.I2C3,
        p.PH7,
        p.PH8,
        Hertz(50_000), // Reduced speed for reliability
        Default::default(),
    );

    info!("I2C Initialized at 50kHz");
    info!("Beginning full diagnostic scan...");

    let mut data = [0u8, 1];
    // Try to find ft5336 touch screen controller
    // For screen we can connected via LCD-TFT controller
    for addr in 0u8..=0xFF {
        // Try write operation first (more reliable for many devices)
        let write_result = i2c.blocking_write_read(addr, &[], &mut data);

        match write_result {
            Ok(()) => info!("Whoami addr : 0x{:02X} data : {}", addr, data[0]),
            Err(Error::Timeout) => error!("Operation timed out"),
            Err(e) => error!("I2c Error: {:?}", e),
        }
        // Small delay between probes
        Timer::after_millis(5).await;
    }

    let mut delay = Delay;

    let mut touch = ft5336::Ft5336::new(&i2c, 0x38, &mut delay).unwrap();

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
                Ok(n) => info!("Touch: {}x{} - weight: {} misc: {}", n.x, n.y, n.weight, n.misc),
            }
        }

        led.set_high();
        info!("On");
        Timer::after_millis(300).await;
        led.set_low();
        info!("Off");
        Timer::after_millis(300).await;
    }
}
