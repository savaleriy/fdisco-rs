#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, Resolution, SampleTime};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

use embassy_stm32::gpio;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::{Duration, Ticker};


const VREFINT_CAL_MV: u32 = 1200;



#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting float-voltage ADC example...");

    let p = embassy_stm32::init(Default::default());
    let mut adc : Adc = Adc::new(p.ADC1);
    adc.set_resolution(Resolution::BITS12);
    adc.set_sample_time(SampleTime::CYCLES480);

    let mut ch_pa0 = p.PA0;

    // Stabilize VrefInt
    Timer::after(Duration::from_millis(10)).await;

    let dt = 100 * 1_000_000;
    let k = 1.003;

    loop {
        // Read raw VREFINT value
        let raw_vref: u16 = {
            let mut vref = adc.enable_vrefint();
            adc.read(&mut vref)
        };

        // Read external channels
        let raw_pa0: u16 = adc.read(&mut ch_pa0);

        // Calculate actual VDDA in volts
        // Using ratio: raw_vref = VREFINT / VDDA * max_adc
        let vdda_volts = (VREFINT_CAL_MV as f32 / 1000.0) * (u16::MAX as f32 / raw_vref as f32);

        // Convert raw ADC readings to volts
        let v_pa0_volts = (raw_pa0 as f32 / u16::MAX as f32) * vdda_volts;

        info!("VDDA = {} V | PA0 = {} V", raw_vref, v_pa0_volts);

        Timer::after(Duration::from_millis(200)).await;
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn toggle_led(control: Sender<'static, ThreadModeRawMutex, LedState, 64>, delay: Duration) {
    let mut ticker = Ticker::every(delay);
    loop {
        control.send(LedState::Toggle).await;
        info!("Toggle");
        ticker.next().await;
    }
}
