#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::rng::Rng;
use rand_core::RngCore;
use embassy_stm32::time::mhz;
use embassy_time::Timer;
use embassy_stm32::{bind_interrupts, peripherals, rng, Config};

use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<peripherals::RNG>;
});


#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {

    use embassy_stm32::rcc::{
        AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv,
        PllRDiv, PllSource, Sysclk,
    };

 
    let mut config = embassy_stm32::Config::default();
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV4;
    config.rcc.apb2_pre = APBPrescaler::DIV2;

    // HSE is on and ready
    config.rcc.hse = Some(Hse {
        freq: mhz(25),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll_src = PllSource::HSE;

    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV25,  // PLLM
        mul: PllMul::MUL400,       // PLLN
        divp: Some(PllPDiv::DIV2), // SYSCLK = 400/2 = 200 MHz
        divq: Some(PllQDiv::DIV9), // PLLQ = 400/9 = 44.44 MHz
        divr: None,
    });

    // This seems to be working, the values in the RCC.PLLSAICFGR are correct according to the debugger. Also on and ready according to CR
    config.rcc.pllsai = Some(Pll {
        prediv: PllPreDiv::DIV25,  // Actually ignored
        mul: PllMul::MUL384,       // PLLN
        divp: Some(PllPDiv::DIV8), // PLLP
        divq: Some(PllQDiv::DIV2), // PLLQ
        divr: Some(PllRDiv::DIV5), // PLLR
    });

    // PLLI2S
    config.rcc.plli2s = Some(Pll {
        prediv: PllPreDiv::DIV25,  // Actually ignored
        mul: PllMul::MUL100,       // PLLN
        divp: Some(PllPDiv::DIV2), // PLLP
        divq: Some(PllQDiv::DIV2), // PLLQ
        divr: Some(PllRDiv::DIV2), // PLLR (I2S PLLR is always 2)
    });

    let p = embassy_stm32::init(config);

    info!("Hello World!");

    // Generate random seed.
    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0; 8];
    rng.async_fill_bytes(&mut seed).await.unwrap();
    // This is doesnt work and i dont know why
    // rng.fill_bytes(&mut seed);
    let seed = u64::from_le_bytes(seed);

    info!("Network task initialized");

    loop {
        // Generate a random number each iteration
        let mut random_bytes = [0u8; 4]; // 4 bytes for a u32
        rng.async_fill_bytes(&mut random_bytes).await.unwrap();
        let random_number = u32::from_le_bytes(random_bytes);
        
        info!("Random number: {}", random_number);
        Timer::after_secs(1).await;
    }
}
