#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");

    let mut led = Output::new(p.PI1, Level::High, Speed::Low);

    loop {
        info!("high");
        led.set_high();
        Timer::after_millis(300).await;

        info!("low");
        led.set_low();
        Timer::after_millis(300).await;
    }
}

// #![no_std]
// #![no_main]

// use defmt::*;
// use embassy_executor::Spawner;
// use embassy_stm32::gpio::{Level, Output, Speed};

// use embassy_stm32::rcc::{
//     AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllRDiv,
//     PllSource, Sysclk,
// };
// use embassy_stm32::{
//     i2c::{I2c},
//     time::{Hertz},
// };
// use embassy_stm32::mode::Blocking;

// use embassy_time::{Timer, Delay};

// use embassy_stm32::time::mhz;

// use ft5336;

// use {defmt_rtt as _, panic_probe as _};

// // Basic Blink Task
// #[embassy_executor::task]
// async fn blink(mut led: Output<'static>) {
//     loop {
//         info!("LED on");
//         led.set_high();
//         Timer::after_millis(300).await;

//         info!("LED off");
//         led.set_low();
//         Timer::after_millis(300).await;
//     }
// }

// #[embassy_executor::task]
// async fn catch_touch(
//     mut touch: ft5336::Ft5336<'static, I2c<'static, Blocking>>,
//     mut i2c: I2c<'static, Blocking>,
// ) {
//     loop {
//         let t = touch.detect_touch(&mut i2c);
//         let mut num: u8 = 0;
//         match t {
//             Err(e) => error!("Error {} from fetching number of touches", e),
//             Ok(n) => {
//                 num = n;
//                 if num != 0 {
//                     info!("Number of touches: {}", num)
//                 };
//             }
//         }
//         if num > 0 {
//             let t = touch.get_touch(&mut i2c, 1);
//             match t {
//                 Err(_e) => error!("Error fetching touch data"),
//                 Ok(n) => info!("Touch: {}x{} - weight: {} misc: {}", n.x, n.y, n.weight, n.misc),
//             }
//         }

//         Timer::after_millis(50).await;
//     }
// }

// // Setup hardware
// fn initialize_hardware() -> embassy_stm32::Config {
//     let mut config = embassy_stm32::Config::default();
//     config.rcc.sys = Sysclk::PLL1_P;
//     config.rcc.ahb_pre = AHBPrescaler::DIV1;
//     config.rcc.apb1_pre = APBPrescaler::DIV4;
//     config.rcc.apb2_pre = APBPrescaler::DIV2;

//     // HSE is on and ready
//     config.rcc.hse = Some(Hse {
//         freq: mhz(25),
//         mode: HseMode::Oscillator,
//     });
//     config.rcc.pll_src = PllSource::HSE;

//     config.rcc.pll = Some(Pll {
//         prediv: PllPreDiv::DIV25,  // PLLM
//         mul: PllMul::MUL400,       // PLLN
//         divp: Some(PllPDiv::DIV2), // SYSCLK = 400/2 = 200 MHz
//         divq: Some(PllQDiv::DIV9), // PLLQ = 400/9 = 44.44 MHz
//         divr: None,
//     });

//     // This seems to be working, the values in the RCC.PLLSAICFGR are correct according to the debugger. Also on and ready according to CR
//     config.rcc.pllsai = Some(Pll {
//         prediv: PllPreDiv::DIV25,  // Actually ignored
//         mul: PllMul::MUL384,       // PLLN
//         divp: Some(PllPDiv::DIV8), // PLLP
//         divq: Some(PllQDiv::DIV2), // PLLQ
//         divr: Some(PllRDiv::DIV5), // PLLR
//     });

//     // PLLI2S
//     config.rcc.plli2s = Some(Pll {
//         prediv: PllPreDiv::DIV25,  // Actually ignored
//         mul: PllMul::MUL100,       // PLLN
//         divp: Some(PllPDiv::DIV2), // PLLP
//         divq: Some(PllQDiv::DIV2), // PLLQ
//         divr: Some(PllRDiv::DIV2), // PLLR (I2S PLLR is always 2)
//     });

//     return config;
// }

// static mut DELAY: Delay = Delay;

// #[embassy_executor::main]
// async fn main(spawner: Spawner) {
//     let config = initialize_hardware();
//     let p = embassy_stm32::init(config);
//     info!("Hello World!");

//     let led = Output::new(p.PI1, Level::Low, Speed::Low);

//     let i2c = I2c::new_blocking(
//         p.I2C3,
//         p.PH7,
//         p.PH8,
//         Hertz(50_000),
//         Default::default(),
//     );

//     let delay_ref: &'static mut Delay = unsafe { &mut DELAY };
//     let touch = ft5336::Ft5336::new(&i2c, 0x38, delay_ref).unwrap();

//     spawner.spawn(blink(led)).unwrap();
//     spawner.spawn(catch_touch(touch, i2c)).unwrap();
// }
