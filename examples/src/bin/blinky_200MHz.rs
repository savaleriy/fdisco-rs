#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    init,
    rcc::{
        AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv,
        PllRDiv, PllSource, Sysclk,
    },
    time::mhz,
    Config,
};
use embassy_time::{Duration, Timer};
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Blinky 200 MHz example");

    let mut config = Config::default();

    // Enable external 25 MHz oscillator
    config.rcc.hse = Some(Hse {
        freq: mhz(25),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll_src = PllSource::HSE;

    // PLL1 for SYSCLK = 400 / 2 = 200 MHz
    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV25,  // PLLM
        mul: PllMul::MUL400,       // PLLN
        divp: Some(PllPDiv::DIV2), // PLLP -> SYSCLK
        divq: Some(PllQDiv::DIV9), // PLLQ
        divr: None,
    });

    // Set SYSCLK source to PLL1_P output
    config.rcc.sys = Sysclk::PLL1_P;

    // Optional: Set bus dividers to match peripheral limits
    config.rcc.ahb_pre = AHBPrescaler::DIV1; // 200 MHz
    config.rcc.apb1_pre = APBPrescaler::DIV4; // Max 50 MHz
    config.rcc.apb2_pre = APBPrescaler::DIV2; // Max 100 MHz

    // Optional: PLLSAI and PLLI2S setup
    config.rcc.pllsai = Some(Pll {
        prediv: PllPreDiv::DIV25,
        mul: PllMul::MUL384,
        divp: Some(PllPDiv::DIV8),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV5),
    });

    config.rcc.plli2s = Some(Pll {
        prediv: PllPreDiv::DIV25,
        mul: PllMul::MUL100,
        divp: Some(PllPDiv::DIV2),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV2),
    });

    let p = init(config);

    info!("Running at SYSCLK = 200 MHz");

    let mut led = Output::new(p.PI1, Level::Low, Speed::Low);

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(250)).await;
        led.set_low();
        Timer::after(Duration::from_millis(250)).await;
    }
}
