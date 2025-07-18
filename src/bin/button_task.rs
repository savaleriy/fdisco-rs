#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Pull;
use embassy_stm32::init;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
async fn user_btn_task(mut user_btn: ExtiInput<'static>) {
    let mut is_high = false;
    info!("Press the USER button...");

    loop {
        let any_edge = user_btn.wait_for_any_edge();
        let timeout = Timer::after(Duration::from_millis(1000));

        // the timeout is here in case of a data race between the last button check
        // and beginning the wait for an edge change
        match select(any_edge, timeout).await {
            Either::First(_) => {}
            Either::Second(_) => {}
        };

        if user_btn.is_high() != is_high {
            is_high = !is_high;
            info!("Button is pressed: {}", is_high);
            // debounce
            Timer::after(Duration::from_millis(50)).await;
        }

        // check button state again as the button may have been
        // released (and remained released) within the debounce period
        if user_btn.is_high() != is_high {
            is_high = !is_high;
            info!("Button is pressed: {}", is_high);
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = init(Default::default());
    info!("Button via Task example");

    let button = ExtiInput::new(p.PI11, p.EXTI11, Pull::Down);

    spawner.spawn(user_btn_task(button)).unwrap();
}
