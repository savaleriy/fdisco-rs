use embassy_stm32::peripherals::*;
use embassy_stm32::fmc::Fmc;
use stm32_fmc::Sdram;

pub struct SdramPins {
    pub a0: PF0,
    pub a1: PF1,
    pub a2: PF2,
    pub a3: PF3,
    pub a4: PF4,
    pub a5: PF5,
    pub a6: PF12,
    pub a7: PF13,
    pub a8: PF14,
    pub a9: PF15,
    pub a10: PG0,
    pub a11: PG1,
    pub ba0: PG4,
    pub ba1: PG5,

    pub d0: PD14,
    pub d1: PD15,
    pub d2: PD0,
    pub d3: PD1,
    pub d4: PE7,
    pub d5: PE8,
    pub d6: PE9,
    pub d7: PE10,
    pub d8: PE11,
    pub d9: PE12,
    pub d10: PE13,
    pub d11: PE14,
    pub d12: PE15,
    pub d13: PD8,
    pub d14: PD9,
    pub d15: PD10,

    pub nbl0: PE0,
    pub nbl1: PE1,

    pub sdclk: PG8,
    pub sdnwe: PG15,
    pub sdcke0: PH3,
    pub sdnc: PC3,
    pub sdne0: PH5,
    pub sdnras: PF11,
}

mod mt48lc4m32b2_6 {
    use stm32_fmc::{SdramChip, SdramConfiguration, SdramTiming};

    const BURST_LENGTH_1: u16 = 0x0000;
    const BURST_TYPE_SEQUENTIAL: u16 = 0x0000;
    const CAS_LATENCY_3: u16 = 0x0030;
    const OPERATING_MODE_STANDARD: u16 = 0x0000;
    const WRITEBURST_MODE_SINGLE: u16 = 0x0200;

    #[derive(Clone, Copy, Debug, PartialEq)]
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

pub fn init_sdram<'d>(
    fmc: FMC,
    pins: SdramPins,
) -> Sdram<Fmc<'d, FMC>, mt48lc4m32b2_6::Mt48lc4m32b2> {
    Fmc::sdram_a12bits_d16bits_4banks_bank1(
        fmc,
        pins.a0, pins.a1, pins.a2, pins.a3, pins.a4, pins.a5,
        pins.a6, pins.a7, pins.a8, pins.a9,
        pins.a10, pins.a11,
        pins.ba0, pins.ba1,
        pins.d0, pins.d1, pins.d2, pins.d3,
        pins.d4, pins.d5, pins.d6, pins.d7,
        pins.d8, pins.d9, pins.d10, pins.d11,
        pins.d12, pins.d13, pins.d14, pins.d15,
        pins.nbl0, pins.nbl1,
        pins.sdnc, pins.sdclk, pins.sdnwe, pins.sdcke0, pins.sdnras, pins.sdne0,
        mt48lc4m32b2_6::Mt48lc4m32b2,
    )
}
