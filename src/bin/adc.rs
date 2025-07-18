//! This is example of how we can get data from adc.
//! So in feature we can create some task that can work with it.
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, Resolution, SampleTime};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

const VREFINT_CAL_MV: u32 = 1200; // Typical internal ref voltage, in millivolts

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting float-voltage ADC example...");

    let p = embassy_stm32::init(Default::default());
    let mut adc = Adc::new(p.ADC1);
    adc.set_resolution(Resolution::BITS12);
    adc.set_sample_time(SampleTime::CYCLES480);

    let mut ch_pa0 = p.PA0;

    // Stabilize VrefInt
    Timer::after(Duration::from_millis(10)).await;

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
