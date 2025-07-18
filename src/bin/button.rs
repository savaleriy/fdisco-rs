#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use {defmt_rtt as _, panic_probe as _};


#[embassy_executor::main]
async fn main(_spawner: Spawner){
    info!("Button example!");

    let p = embassy_stm32::init(Default::default());

    let button = ExtiInput::new(p.PI11, p.EXTI11, Pull::Down);
    let mut led1 = Output::new(p.PI1, Level::High, Speed::Low);

    loop {
        if button.is_high() {
            info!("high");
            led1.set_high();
        } else {
            info!("low");
            led1.set_low();
        }
    }
}
