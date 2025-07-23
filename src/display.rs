use embassy_stm32::{
    pac::RCC,
    gpio::{AfType, Flex, Level, Output, OutputType, Speed},
    ltdc::Ltdc,
};

use embassy_stm32::peripherals::*;

pub struct DisplayPins {
    pub r0: PI15,
    pub r1: PJ0,
    pub r2: PJ1,
    pub r3: PJ2,
    pub r4: PJ3,
    pub r5: PJ4,
    pub r6: PJ5,
    pub r7: PJ6,

    pub g0: PJ7,
    pub g1: PJ8,
    pub g2: PJ9,
    pub g3: PJ10,
    pub g4: PJ11,
    pub g5: PK0,
    pub g6: PK1,
    pub g7: PK2,

    pub b0: PE4,
    pub b1: PJ13,
    pub b2: PJ14,
    pub b3: PJ15,
    pub b4: PG12,
    pub b5: PK4,
    pub b6: PK5,
    pub b7: PK6,

    pub hsync: PI10,
    pub vsync: PI9,
    pub clk: PI14,
    pub de: PK7,

    pub lcd_en: PI12,
    pub backlight: PK3,
}

/// Initializes the LTDC display controller and returns the configured instance
pub fn init_display(pins: DisplayPins, ltdc_periph: embassy_stm32::peripherals::LTDC) {
    let data_af = AfType::output(OutputType::PushPull, Speed::Low);
    // Red
    Flex::new(pins.r0).set_as_af_unchecked(14, data_af);
    Flex::new(pins.r1).set_as_af_unchecked(14, data_af);
    Flex::new(pins.r2).set_as_af_unchecked(14, data_af);
    Flex::new(pins.r3).set_as_af_unchecked(14, data_af);
    Flex::new(pins.r4).set_as_af_unchecked(14, data_af);
    Flex::new(pins.r5).set_as_af_unchecked(14, data_af);
    Flex::new(pins.r6).set_as_af_unchecked(14, data_af);
    Flex::new(pins.r7).set_as_af_unchecked(14, data_af);

    // Green
    Flex::new(pins.g0).set_as_af_unchecked(14, data_af);
    Flex::new(pins.g1).set_as_af_unchecked(14, data_af);
    Flex::new(pins.g2).set_as_af_unchecked(14, data_af);
    Flex::new(pins.g3).set_as_af_unchecked(14, data_af);
    Flex::new(pins.g4).set_as_af_unchecked(14, data_af);
    Flex::new(pins.g5).set_as_af_unchecked(14, data_af);
    Flex::new(pins.g6).set_as_af_unchecked(14, data_af);
    Flex::new(pins.g7).set_as_af_unchecked(14, data_af);

    // Blue
    Flex::new(pins.b0).set_as_af_unchecked(14, data_af);
    Flex::new(pins.b1).set_as_af_unchecked(14, data_af);
    Flex::new(pins.b2).set_as_af_unchecked(14, data_af);
    Flex::new(pins.b3).set_as_af_unchecked(14, data_af);
    Flex::new(pins.b4).set_as_af_unchecked(14, data_af);
    Flex::new(pins.b5).set_as_af_unchecked(14, data_af);
    Flex::new(pins.b6).set_as_af_unchecked(14, data_af);
    Flex::new(pins.b7).set_as_af_unchecked(14, data_af);

    // Control Signals
    Flex::new(pins.hsync).set_as_af_unchecked(14, data_af);
    Flex::new(pins.vsync).set_as_af_unchecked(14, data_af);
    Flex::new(pins.clk).set_as_af_unchecked(14, data_af);
    Flex::new(pins.de).set_as_af_unchecked(14, data_af);

    // Display enable & backlight control
    let _lcd_en = Output::new(pins.lcd_en, Level::High, Speed::Low);
    let _backlight = Output::new(pins.backlight, Level::High, Speed::Low);

    // Initialize LTDC
    let mut ltdc = Ltdc::new(ltdc_periph);

    critical_section::with(|_cs| {
        // RM says the pllsaidivr should only be changed when pllsai is off. But this could have other unintended side effects. So let's just give it a try like this.
        // According to the debugger, this bit gets set, anyway.
        RCC.dckcfgr1()
            .modify(|w| w.set_pllsaidivr(embassy_stm32::pac::rcc::vals::Pllsaidivr::DIV8));
    });

    embassy_stm32::rcc::enable_and_reset::<embassy_stm32::peripherals::LTDC>();

    ltdc.disable();

    use embassy_stm32::pac::LTDC;

    // Set the LTDC to 480x272
    LTDC.gcr().modify(|w| {
        w.set_hspol(embassy_stm32::pac::ltdc::vals::Hspol::ACTIVELOW);
        w.set_vspol(embassy_stm32::pac::ltdc::vals::Vspol::ACTIVELOW);
        w.set_depol(embassy_stm32::pac::ltdc::vals::Depol::ACTIVELOW);
        w.set_pcpol(embassy_stm32::pac::ltdc::vals::Pcpol::RISINGEDGE);
    });

    // Set Sync signals
    LTDC.sscr().write(|w| {
        w.set_hsw(41);
        w.set_vsh(10);
    });

    // Set Accumulated Back porch
    LTDC.bpcr().modify(|w| {
        w.set_ahbp(53);
        w.set_avbp(11);
    });

    // Set Accumulated Active Width
    LTDC.awcr().modify(|w| {
        w.set_aah(283);
        w.set_aaw(533);
    });

    // Set Total Width
    LTDC.twcr().modify(|w| {
        w.set_totalh(285);
        w.set_totalw(565);
    });

    // Set the background color value
    LTDC.bccr().modify(|w| {
        w.set_bcred(0);
        w.set_bcgreen(0);
        w.set_bcblue(0)
    });

    // Enable the Transfer Error and FIFO underrun interrupts
    LTDC.ier().modify(|w| {
        w.set_terrie(true);
        w.set_fuie(true);
    });
}
