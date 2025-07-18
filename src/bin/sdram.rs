// src/bin/sdram.rs
// https://docs.rs/stm32-fmc/latest/stm32_fmc/devices/mt48lc4m32b2_6/struct.Mt48lc4m32b2.html
//
#![no_std]
#![no_main]

use defmt::*;

use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::fmc::Fmc;
use embassy_stm32::time::mhz;
use embassy_time::Timer;
use embedded_alloc::TlsfHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllRDiv,
    PllSource, Sysclk,
};
extern crate alloc;

use alloc::boxed::Box;
use core::slice;

// --------------------------------------------------
// SDRAM chip definition
// --------------------------------------------------
mod mt48lc4m32b2_6 {
    use stm32_fmc::{SdramChip, SdramConfiguration, SdramTiming};

    const BURST_LENGTH_1: u16 = 0x0000;
    const BURST_TYPE_SEQUENTIAL: u16 = 0x0000;
    const CAS_LATENCY_3: u16 = 0x0030;
    const OPERATING_MODE_STANDARD: u16 = 0x0000;
    const WRITEBURST_MODE_SINGLE: u16 = 0x0200;

    #[derive(Clone, Copy, Debug)]
    pub struct Mt48lc4m32b2;

    impl SdramChip for Mt48lc4m32b2 {
        const MODE_REGISTER: u16 = BURST_LENGTH_1
            | BURST_TYPE_SEQUENTIAL
            | CAS_LATENCY_3
            | OPERATING_MODE_STANDARD
            | WRITEBURST_MODE_SINGLE;

        const TIMING: SdramTiming = SdramTiming {
            startup_delay_ns: 100_000,
            max_sd_clock_hz: 100_000_000,
            refresh_period_ns: 15_625,
            mode_register_to_active: 2,
            exit_self_refresh: 7,
            active_to_precharge: 4,
            row_cycle: 7,
            row_precharge: 2,
            row_to_column: 2,
        };

        const CONFIG: SdramConfiguration = SdramConfiguration {
            column_bits: 8,
            row_bits: 12,
            memory_data_width: 16,
            internal_banks: 4,
            cas_latency: 3,
            write_protection: false,
            read_burst: true,
            read_pipe_delay_cycles: 0,
        };
    }
}

// --------------------------------------------------
// Main entry
// --------------------------------------------------
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("SDRAM example");

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

    // Config SDRAM
    // ----------------------------------------------------------
    // Configure MPU for external SDRAM (64 Mbit = 8 Mbyte)
    // MPU is disabled by default!
    const SDRAM_SIZE: usize = 8 * 1024 * 1024; // 8 MB

    #[rustfmt::skip]
    let mut sdram = Fmc::sdram_a12bits_d16bits_4banks_bank1(
        p.FMC,
        // A0-A11
        p.PF0, p.PF1, p.PF2, p.PF3, p.PF4, p.PF5, p.PF12, p.PF13, p.PF14, p.PF15, p.PG0, p.PG1,
        // BA0-BA1
        p.PG4, p.PG5,
        // D0-D15
        p.PD14, p.PD15, p.PD0, p.PD1, p.PE7, p.PE8, p.PE9, p.PE10, p.PE11, p.PE12, p.PE13, p.PE14, p.PE15, p.PD8, p.PD9, p.PD10,
        // NBL0 - NBL1
        p.PE0, p.PE1,
        p.PC3,  // SDCKE0
        p.PG8,  // SDCLK
        p.PG15, // SDNCAS
        p.PH3,  // SDNE0 (!CS)
        p.PF11, // SDRAS
        p.PH5,  // SDNWE
        mt48lc4m32b2_6::Mt48lc4m32b2 {},
    );

    let mut delay = embassy_time::Delay;

        // Initialise controller and SDRAM
        let ram_ptr: *mut u8 = sdram.init(&mut delay) as *mut _;

        info!("SDRAM Initialized at {:x}", ram_ptr as usize);

    unsafe {
        // Convert raw pointer to slice
        // Move Heap to SDRAM!
        HEAP.init(ram_ptr as usize, SDRAM_SIZE)

    };

    // Dynamic Vec allocation backed by SDRAM
    let mut v: alloc::vec::Vec<u32> = alloc::vec::Vec::new();
    v.reserve(100);
    for i in 0..100 {
        v.push(i * 2);
    }
    info!("Vec in SDRAM length {}", v.len());

    // Boxed allocation in SDRAM
    let boxed = Box::new(0xDEADBEEF_u32);
    info!("Boxed SDRAM value {=u32}", *boxed);

    {
        // SDRAM-backed global allocator (TlsfHeap)
        //   to allocate a Vec<f32> of size N x N in SDRAM
        const N: usize = 1_000;                // 1 000 x 1 000 ≃ 1 000 000 elements
        let total_elems = N * N;               // = 1 000 000
        let mut mat: alloc::vec::Vec<f32> = alloc::vec::Vec::with_capacity(total_elems);

        // Log where the matrix lives in SDRAM:
        info!("Allocating matrix {} x {} in SDRAM", N, N);
        info!("Matrix buffer at address 0x{:08X}", mat.as_ptr() as usize);

        for row in 0..N {
            for col in 0..N {
                mat.push((row as f32) + (col as f32) / (N as f32));
            }
        }

        info!("Matrix initialized");

        // Compute a simple checksum (sum of all elements) as a sanity check
        let checksum: f32 = mat.iter().copied().sum();
        info!("Matrix checksum = {=f32}", checksum);

        let corners = [
            mat[0],                             // (0,0)
            mat[N - 1],                         // (0,999)
            mat[total_elems - N],               // (999,0)
            mat[total_elems - 1],               // (999,999)
        ];
        info!("Corners = [{=f32}, {=f32}, {=f32}, {=f32}]", corners[0], corners[1], corners[2], corners[3]);
    };

    loop {
        Timer::after(embassy_time::Duration::from_secs(1)).await;
        info!("Heartbeat.");
    }
}
