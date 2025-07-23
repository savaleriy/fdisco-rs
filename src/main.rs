/*#![no_std]
#![no_main]

// Graphics Driver

use embedded_graphics::{
    geometry,
    pixelcolor::{IntoStorage, Rgb888},
    Pixel,
};

pub struct DisplayBuffer<'a> {
    pub buf: &'a mut [u32],
    pub width: i32,
    pub height: i32,
}

// To work with Embedded Graphics Crate we should implement DrawTarget
// For as Display
// Implement DrawTarget for
impl embedded_graphics::draw_target::DrawTarget for DisplayBuffer<'_> {
    type Color = Rgb888;
    type Error = ();

    /// Draw a pixel
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let Pixel(point, color) = pixel;
            let argb = color.into_storage() | 0xFF00_0000u32;

            if point.x >= 0 && point.y >= 0 && point.x < self.width && point.y < self.height {
                let index = point.y * self.width + point.x;
                self.buf[index as usize] = argb;
            } else {
                // Ignore invalid points
            }
        }

        Ok(())
    }
}
impl geometry::OriginDimensions for DisplayBuffer<'_> {
    /// Return the size of the display
    fn size(&self) -> geometry::Size {
        geometry::Size::new(self.width as u32, self.height as u32)
    }
}

impl DisplayBuffer<'_> {
    /// Clears the buffer
    pub fn clear(&mut self) {
        let pixels = self.width * self.height;

        for a in self.buf[..pixels as usize].iter_mut() {
            *a = 0xFF00_0000u32; // Solid black
        }
    }
}

// SDRAM driver
use embedded_alloc::TlsfHeap as Heap;
#[global_allocator]
static HEAP: Heap = Heap::empty();

extern crate alloc;

use alloc::boxed::Box;

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

// Touch detector
use embassy_stm32::mode::Blocking;
use embassy_stm32::{i2c::I2c, time::Hertz};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Timer};
use embedded_graphics::geometry::Point;
use ft5336::Ft5336;
//Declate a channel of 1 Point
static SHARED: Channel<ThreadModeRawMutex, Point, 1> = Channel::new();
static mut DELAY: Delay = Delay;

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::fmc::Fmc;
use embassy_stm32::gpio::{AfType, Flex, Level, Output, OutputType, Speed};
use embassy_stm32::ltdc::{
    B0Pin, B1Pin, B2Pin, B3Pin, B4Pin, B5Pin, B6Pin, B7Pin, ClkPin, DePin, G0Pin, G1Pin, G2Pin,
    G3Pin, G4Pin, G5Pin, G6Pin, G7Pin, HsyncPin, Ltdc, R0Pin, R1Pin, R2Pin, R3Pin, R4Pin, R5Pin,
    R6Pin, R7Pin, VsyncPin,
};
use embassy_stm32::pac::ltdc::vals::{Bf1, Bf2, Imr, Pf};
use embassy_stm32::pac::RCC;
use embassy_stm32::time::mhz;

use embedded_graphics::geometry::{Dimensions, Size};
use embedded_graphics::mono_font::iso_8859_14::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::primitives::{Primitive, PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_layout::align::{horizontal, vertical, Align};
use embedded_layout::layout::linear::LinearLayout;
use embedded_layout::object_chain::Chain;

use {defmt_rtt as _, panic_probe as _};

#[derive(Clone, Copy, defmt::Format)]
enum ButtonEvent {
    D0,
    D1,
    D2,
    D3,
}

static BUTTON_EVENTS: Channel<ThreadModeRawMutex, ButtonEvent, 32> = Channel::new();


// GUI Buttons
struct Button<'a> {
    area: Rectangle,
    text: &'a str,
    text_style: MonoTextStyle<'a, Rgb888>,
    is_pressed: bool,
    pressed_color: Rgb888,
    released_color: Rgb888,
}

impl<'a> Button<'a> {
    fn new(
        position: Point,
        size: Size,
        text: &'a str,
        text_style: MonoTextStyle<'a, Rgb888>,
    ) -> Self {
        Self {
            area: Rectangle::new(position, size),
            text,
            text_style,
            is_pressed: false,
            pressed_color: Rgb888::GREEN,
            released_color: Rgb888::WHITE,
        }
    }

    fn check_touch(&mut self, point: Point) -> bool {
        self.area.contains(point)
    }

    fn draw(&self, display: &mut DisplayBuffer) {
        let fill_color = if self.is_pressed {
            Rgb888::GREEN
        } else {
            Rgb888::RED
        };

        let text = if self.is_pressed {
            alloc::format!("{}: ON", self.text)
        } else {
            alloc::format!("{}: OFF", self.text)
        };

        let text_style = MonoTextStyle::new(&FONT_10X20, Rgb888::BLACK);

        self.area
            .into_styled(PrimitiveStyle::with_fill(fill_color))
            .draw(display)
            .unwrap();

        let text_size = Text::new(&text, Point::zero(), text_style)
            .bounding_box()
            .size;

        let text_position = Point::new(
            self.area.top_left.x + (self.area.size.width as i32 - text_size.width as i32) / 2,
            self.area.top_left.y + (self.area.size.height as i32 - text_size.height as i32) / 2,
        );

        Text::new(&text, text_position, text_style)
            .draw(display)
            .unwrap();
    }
}

#[embassy_executor::task()]
async fn display_task() -> ! {
    use embassy_stm32::pac::LTDC;

    info!("Display task started");

    const LCD_X_SIZE: u16 = 480;
    const LCD_Y_SIZE: u16 = 272;

    /* Initialize the LCD pixel width and pixel height */
    const WINDOW_X0: u16 = 0;
    const WINDOW_X1: u16 = LCD_X_SIZE; // 480 for ferris
    const WINDOW_Y0: u16 = 0;
    const WINDOW_Y1: u16 = LCD_Y_SIZE; // 800 for ferris
    const PIXEL_FORMAT: Pf = Pf::ARGB8888;
    //const FBStartAdress: u16 = FB_Address;
    const ALPHA: u8 = 255;
    const ALPHA0: u8 = 0;
    const BACKCOLOR_BLUE: u8 = 0;
    const BACKCOLOR_GREEN: u8 = 0;
    const BACKCOLOR_RED: u8 = 0;
    const IMAGE_WIDTH: u16 = LCD_X_SIZE; // 480 for ferris
    const IMAGE_HEIGHT: u16 = LCD_Y_SIZE; // 800 for ferris

    const PIXEL_SIZE: u8 = match PIXEL_FORMAT {
        Pf::ARGB8888 => 4,
        Pf::RGB888 => 3,
        Pf::ARGB4444 | Pf::RGB565 | Pf::ARGB1555 | Pf::AL88 => 2,
        _ => 1,
    };

    // Configure the horizontal start and stop position
    LTDC.layer(0).whpcr().write(|w| {
        w.set_whstpos(LTDC.bpcr().read().ahbp() + 1 + WINDOW_X0);
        w.set_whsppos(LTDC.bpcr().read().ahbp() + WINDOW_X1);
    });

    // Configures the vertical start and stop position
    LTDC.layer(0).wvpcr().write(|w| {
        w.set_wvstpos(LTDC.bpcr().read().avbp() + 1 + WINDOW_Y0);
        w.set_wvsppos(LTDC.bpcr().read().avbp() + WINDOW_Y1);
    });

    // Specify the pixel format
    LTDC.layer(0).pfcr().write(|w| w.set_pf(PIXEL_FORMAT));

    // Configures the default color values as zero
    LTDC.layer(0).dccr().modify(|w| {
        w.set_dcblue(BACKCOLOR_BLUE);
        w.set_dcgreen(BACKCOLOR_GREEN);
        w.set_dcred(BACKCOLOR_RED);
        w.set_dcalpha(ALPHA0);
    });

    // Specifies the constant ALPHA value
    LTDC.layer(0).cacr().write(|w| w.set_consta(ALPHA));

    // Specifies the blending factors
    LTDC.layer(0).bfcr().write(|w| {
        w.set_bf1(Bf1::CONSTANT);
        w.set_bf2(Bf2::CONSTANT);
    });

    // Allocate a buffer for the display on the heap
    const DISPLAY_BUFFER_SIZE: usize = LCD_X_SIZE as usize * LCD_Y_SIZE as usize;
    let mut display_buffer_1 = Box::<[u32; DISPLAY_BUFFER_SIZE]>::new([0; DISPLAY_BUFFER_SIZE]);
    let mut display_buffer_2 = Box::<[u32; DISPLAY_BUFFER_SIZE]>::new([0; DISPLAY_BUFFER_SIZE]);
    info!(
        "Display buffer allocated at {:x}, {:x}",
        &display_buffer_1[0] as *const _, &display_buffer_2[0] as *const _
    );
    // Create a display buffer
    let mut display_fb1 = DisplayBuffer {
        buf: &mut display_buffer_1.as_mut_slice(),
        width: LCD_X_SIZE as i32,
        height: LCD_Y_SIZE as i32,
    };

    let mut display_fb2 = DisplayBuffer {
        buf: &mut display_buffer_2.as_mut_slice(),
        width: LCD_X_SIZE as i32,
        height: LCD_Y_SIZE as i32,
    };

    let mut display = &mut display_fb1;

    info!(
        "Display buffer allocated at {:x}",
        &display.buf[0] as *const _
    );

    LTDC.layer(0)
        .cfbar()
        .write(|w| w.set_cfbadd(&display.buf[0] as *const _ as u32));

    // Configures the color frame buffer pitch in byte
    LTDC.layer(0).cfblr().write(|w| {
        w.set_cfbp(IMAGE_WIDTH * PIXEL_SIZE as u16);
        w.set_cfbll(((WINDOW_X1 - WINDOW_X0) * PIXEL_SIZE as u16) + 3);
    });

    // Configures the frame buffer line number
    LTDC.layer(0)
        .cfblnr()
        .write(|w| w.set_cfblnbr(IMAGE_HEIGHT));

    // Enable LTDC_Layer by setting LEN bit
    LTDC.layer(0).cr().modify(|w| w.set_len(true));

    //LTDC->SRCR = LTDC_SRCR_IMR;
    LTDC.srcr().modify(|w| w.set_imr(Imr::RELOAD));

    // Delay for 1s
    Timer::after_millis(1000).await;

    // Create a Rectangle from the display's dimensions
    let display_area = display.bounding_box();

    // Disable the layer
    LTDC.layer(0).cr().modify(|w| w.set_len(false));

    // replace the buffer with the new one
    LTDC.layer(0)
        .cfbar()
        .write(|w| w.set_cfbadd(&display.buf[0] as *const _ as u32));

    // Configures the color frame buffer pitch in byte
    LTDC.layer(0).cfblr().write(|w| {
        w.set_cfbp(IMAGE_WIDTH * 4 as u16);
        w.set_cfbll(((WINDOW_X1 - WINDOW_X0) * 4 as u16) + 3);
    });

    // Configures the frame buffer line number
    LTDC.layer(0)
        .cfblnr()
        .write(|w| w.set_cfblnbr(IMAGE_HEIGHT));

    // Use ARGB8888 pixel format
    LTDC.layer(0).pfcr().write(|w| w.set_pf(Pf::ARGB8888));

    // Enable the layer
    LTDC.layer(0).cr().modify(|w| w.set_len(true));

    // Immediately refresh the display
    LTDC.srcr().modify(|w| w.set_imr(Imr::RELOAD));

    // Style objects
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb888::BLUE);

    let text = Text::new("Simple GUI", Point::zero(), text_style);

    // Create buttons
    let mut button1 = Button::new(
        Point::new(100, 60),
        Size::new(100, 50),
        "D0",
        MonoTextStyle::new(&FONT_10X20, Rgb888::BLACK),
    );

    let mut button2 = Button::new(
        Point::new(300, 60),
        Size::new(100, 50),
        "D1",
        MonoTextStyle::new(&FONT_10X20, Rgb888::BLACK),
    );

    let mut button3 = Button::new(
        Point::new(100, 130),
        Size::new(100, 50),
        "D2",
        MonoTextStyle::new(&FONT_10X20, Rgb888::BLACK),
    );

    let mut button4 = Button::new(
        Point::new(300, 130),
        Size::new(100, 50),
        "D3",
        MonoTextStyle::new(&FONT_10X20, Rgb888::BLACK),
    );
    // Inital State
    let mut d0_state = false;
    let mut d1_state = false;
    let mut d2_state = false;
    let mut d3_state = false;

    let mut active_buffer = 0;

    loop {
        // Check for touch events from GUI
        if let Ok(raw_point) = SHARED.try_receive() {
            let point = if raw_point.x > 0 && raw_point.y > 0 {
                Some(raw_point)
            } else {
                None
            };
            info!("Point {} x {}", raw_point.x, raw_point.y);
            if let Some(p) = point {
                if button1.check_touch(p) {
                    info!("Send D0");
                    BUTTON_EVENTS.send(ButtonEvent::D0).await;
                }
                if button2.check_touch(p) {
                    info!("Send D1");
                    BUTTON_EVENTS.send(ButtonEvent::D1).await;
                }
                if button3.check_touch(p) {
                    info!("Send D2");
                    BUTTON_EVENTS.send(ButtonEvent::D2).await;
                }
                if button4.check_touch(p) {
                    info!("Send D3");
                    BUTTON_EVENTS.send(ButtonEvent::D3).await;
                }
            }
        }
        // Grep Status form Hardware
        if let Ok(event) = PIN_STATE_EVENTS.try_receive() {
            match event {
                PinStateEvent::D0(state) => d0_state = state,
                PinStateEvent::D1(state) => d1_state = state,
                PinStateEvent::D2(state) => d2_state = state,
                PinStateEvent::D3(state) => d3_state = state,
            }
        }
        // Update Button State
        button1.is_pressed = d0_state;
        button2.is_pressed = d1_state;
        button3.is_pressed = d2_state;
        button4.is_pressed = d3_state;

        // Switch buffers (double buffering)
        let display = if active_buffer == 0 {
            active_buffer = 1;
            &mut display_fb1
        } else {
            active_buffer = 0;
            &mut display_fb2
        };

        display.clear();

        let layout = LinearLayout::vertical(Chain::new(text))
            .with_alignment(horizontal::Center)
            .arrange()
            .align_to(&display_area, horizontal::Center, vertical::Top)
            .draw(display)
            .unwrap();

        // Draw all buttons
        button1.draw(display);
        button2.draw(display);
        button3.draw(display);
        button4.draw(display);

        // Update LTDC buffer address
        LTDC.layer(0)
            .cfbar()
            .write(|w| w.set_cfbadd(&display.buf[0] as *const _ as u32));

        // Refresh the display
        LTDC.srcr().modify(|w| w.set_imr(Imr::RELOAD));

        Timer::after_millis(120).await;
    }
}


static mut DEBOUNCE_COUNTER: u8 = 0;

#[embassy_executor::task]
async fn catch_touch(
    mut touch: ft5336::Ft5336<'static, I2c<'static, Blocking>>,
    mut i2c: I2c<'static, Blocking>,
) {
    loop {
        let t = touch.detect_touch(&mut i2c);
        match t {
            Err(e) => error!("Error {} from fetching number of touches", e),
            Ok(n) => {
                if n > 0 {
                    let t = touch.get_touch(&mut i2c, 1);
                    match t {
                        Err(_) => error!("Error fetching touch data"),
                        Ok(n) => {
                            // Debounce: only send touch event if not in debounce state
                            if unsafe { DEBOUNCE_COUNTER } == 0 {
                                let p = Point::new(n.y.into(), n.x.into());
                                SHARED.send(p).await;
                                unsafe { DEBOUNCE_COUNTER = 10 }; // Set debounce counter
                            }
                        }
                    }
                }
            }
        }

        // Decrement debounce counter every 50ms
        if unsafe { DEBOUNCE_COUNTER } > 0 {
            unsafe { DEBOUNCE_COUNTER -= 1 };
        }

        Timer::after_millis(50).await;
    }
}

#[derive(Clone, Copy)]
enum PinStateEvent {
    D0(bool),
    D1(bool),
    D2(bool),
    D3(bool),
}

static PIN_STATE_EVENTS: Channel<ThreadModeRawMutex, PinStateEvent, 32> = Channel::new();

#[embassy_executor::task]
async fn buttons_task(
    mut D0: Output<'static>,
    mut D1: Output<'static>,
    mut D2: Output<'static>,
    mut D3: Output<'static>,
) {

    // Init ports on outputs 
    loop {
        let event = BUTTON_EVENTS.receive().await;
        info!("Event {}", event);

        match  event {
            ButtonEvent::D0 => {
                D0.toggle();
                info!("D0 : {}", D0.get_output_level());
                PIN_STATE_EVENTS.send(PinStateEvent::D0(D0.is_set_high())).await;
            }
            ButtonEvent::D1 => {
                D1.toggle();
                info!("D1 : {}", D1.get_output_level());
                PIN_STATE_EVENTS.send(PinStateEvent::D1(D1.is_set_high())).await;
            }
            ButtonEvent::D2 => {
                D2.toggle();
                info!("D2 : {}", D2.get_output_level());
                PIN_STATE_EVENTS.send(PinStateEvent::D2(D2.is_set_high())).await;
            }
            ButtonEvent::D3 => {
                D3.toggle();
                info!("D3 : {}", D3.get_output_level());
                PIN_STATE_EVENTS.send(PinStateEvent::D3(D3.is_set_high())).await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
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
    info!("Starting...");

    // Config SDRAM
    // ----------------------------------------------------------
    // Configure MPU for external SDRAM (64 Mbit = 8 Mbyte)
    // MPU is disabled by default
    const SDRAM_SIZE: usize = 8 * 1024 * 1024;

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

    unsafe {
        // Initialise controller and SDRAM
        let ram_ptr: *mut u32 = sdram.init(&mut delay) as *mut _;

        info!("SDRAM Initialized at {:x}", ram_ptr as usize);

        // Convert raw pointer to slice
        HEAP.init(ram_ptr as usize, SDRAM_SIZE)
    };

    // Test memory
    let mut boxed_int = Box::new(0xdeadbeefu32);

    info!("Boxed int at {:x}", &*boxed_int as *const _);
    info!("Boxed value: {:x}", *boxed_int);

    *boxed_int += 1;

    info!("Boxed value: {:x}", *boxed_int);

    // Configure the LTDC Pins
    const DATA_AF: AfType = AfType::output(OutputType::PushPull, Speed::Low);

    // R: PI15, PJ0..6, 8 bits
    let ltdc_r0_af = p.PI15.af_num();
    let mut ltdc_r0 = Flex::new(p.PI15);
    ltdc_r0.set_as_af_unchecked(ltdc_r0_af, DATA_AF);

    let ltdc_r1_af = p.PJ0.af_num();
    let mut ltdc_r1 = Flex::new(p.PJ0);
    ltdc_r1.set_as_af_unchecked(ltdc_r1_af, DATA_AF);

    let ltdc_r2_af = p.PJ1.af_num();
    let mut ltdc_r2 = Flex::new(p.PJ1);
    ltdc_r2.set_as_af_unchecked(ltdc_r2_af, DATA_AF);

    let ltdc_r3_af = p.PJ2.af_num();
    let mut ltdc_r3 = Flex::new(p.PJ2);
    ltdc_r3.set_as_af_unchecked(ltdc_r3_af, DATA_AF);

    let ltdc_r4_af = p.PJ3.af_num();
    let mut ltdc_r4 = Flex::new(p.PJ3);
    ltdc_r4.set_as_af_unchecked(ltdc_r4_af, DATA_AF);

    let ltdc_r5_af = p.PJ4.af_num();
    let mut ltdc_r5 = Flex::new(p.PJ4);
    ltdc_r5.set_as_af_unchecked(ltdc_r5_af, DATA_AF);

    let ltdc_r6_af = p.PJ5.af_num();
    let mut ltdc_r6 = Flex::new(p.PJ5);
    ltdc_r6.set_as_af_unchecked(ltdc_r6_af, DATA_AF);

    let ltdc_r7_af = p.PJ6.af_num();
    let mut ltdc_r7 = Flex::new(p.PJ6);
    ltdc_r7.set_as_af_unchecked(ltdc_r7_af, DATA_AF);

    // G: PJ7..11, PK0..2, 8 bits
    let ltdc_g0_af = p.PJ7.af_num();
    let mut ltdc_g0 = Flex::new(p.PJ7);
    ltdc_g0.set_as_af_unchecked(ltdc_g0_af, DATA_AF);

    let ltdc_g1_af = p.PJ8.af_num();
    let mut ltdc_g1 = Flex::new(p.PJ8);
    ltdc_g1.set_as_af_unchecked(ltdc_g1_af, DATA_AF);

    let ltdc_g2_af = p.PJ9.af_num();
    let mut ltdc_g2 = Flex::new(p.PJ9);
    ltdc_g2.set_as_af_unchecked(ltdc_g2_af, DATA_AF);

    let ltdc_g3_af = p.PJ10.af_num();
    let mut ltdc_g3 = Flex::new(p.PJ10);
    ltdc_g3.set_as_af_unchecked(ltdc_g3_af, DATA_AF);

    let ltdc_g4_af = p.PJ11.af_num();
    let mut ltdc_g4 = Flex::new(p.PJ11);
    ltdc_g4.set_as_af_unchecked(ltdc_g4_af, DATA_AF);

    let ltdc_g5_af = p.PK0.af_num();
    let mut ltdc_g5 = Flex::new(p.PK0);
    ltdc_g5.set_as_af_unchecked(ltdc_g5_af, DATA_AF);

    let ltdc_g6_af = p.PK1.af_num();
    let mut ltdc_g6 = Flex::new(p.PK1);
    ltdc_g6.set_as_af_unchecked(ltdc_g6_af, DATA_AF);

    let ltdc_g7_af = p.PK2.af_num();
    let mut ltdc_g7 = Flex::new(p.PK2);
    ltdc_g7.set_as_af_unchecked(ltdc_g7_af, DATA_AF);

    // B: PE4, PJ13..15, PG12, PK4..6, 8 bits
    let ltdc_b0_af = p.PE4.af_num();
    let mut ltdc_b0 = Flex::new(p.PE4);
    ltdc_b0.set_as_af_unchecked(ltdc_b0_af, DATA_AF);

    let ltdc_b1_af = p.PJ13.af_num();
    let mut ltdc_b1 = Flex::new(p.PJ13);
    ltdc_b1.set_as_af_unchecked(ltdc_b1_af, DATA_AF);

    let ltdc_b2_af = p.PJ14.af_num();
    let mut ltdc_b2 = Flex::new(p.PJ14);
    ltdc_b2.set_as_af_unchecked(ltdc_b2_af, DATA_AF);

    let ltdc_b3_af = p.PJ15.af_num();
    let mut ltdc_b3 = Flex::new(p.PJ15);
    ltdc_b3.set_as_af_unchecked(ltdc_b3_af, DATA_AF);

    let ltdc_b4_af = B4Pin::af_num(&p.PG12);
    let mut ltdc_b4 = Flex::new(p.PG12);
    ltdc_b4.set_as_af_unchecked(ltdc_b4_af, DATA_AF);

    let ltdc_b5_af = p.PK4.af_num();
    let mut ltdc_b5 = Flex::new(p.PK4);
    ltdc_b5.set_as_af_unchecked(ltdc_b5_af, DATA_AF);

    let ltdc_b6_af = p.PK5.af_num();
    let mut ltdc_b6 = Flex::new(p.PK5);
    ltdc_b6.set_as_af_unchecked(ltdc_b6_af, DATA_AF);

    let ltdc_b7_af = p.PK6.af_num();
    let mut ltdc_b7 = Flex::new(p.PK6);
    ltdc_b7.set_as_af_unchecked(ltdc_b7_af, DATA_AF);

    // HSYNC: PI10
    let ltdc_hsync_af = p.PI10.af_num();
    let mut ltdc_hsync = Flex::new(p.PI10);
    ltdc_hsync.set_as_af_unchecked(ltdc_hsync_af, DATA_AF);

    // VSYNC: PI9
    let ltdc_vsync_af = p.PI9.af_num();
    let mut ltdc_vsync = Flex::new(p.PI9);
    ltdc_vsync.set_as_af_unchecked(ltdc_vsync_af, DATA_AF);

    // CLK: PI14
    let ltdc_clk_af = p.PI14.af_num();
    let mut ltdc_clk = Flex::new(p.PI14);
    ltdc_clk.set_as_af_unchecked(ltdc_clk_af, DATA_AF);

    // DE: PK7
    let ltdc_de_af = p.PK7.af_num();
    let mut ltdc_de = Flex::new(p.PK7);
    ltdc_de.set_as_af_unchecked(ltdc_de_af, DATA_AF);

    // Enable the LCD-TFT controller
    let _lcd_en = Output::new(p.PI12, Level::High, Speed::Low);

    // Enable the backlight
    let _backlight = Output::new(p.PK3, Level::High, Speed::Low);

    let mut ltdc = Ltdc::new(p.LTDC);

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

    // Enable the LTDC
    ltdc.enable();

    // Start the display task
    let spawner = Spawner::for_current_executor().await;

    spawner.spawn(display_task()).unwrap();

    let i2c = I2c::new_blocking(p.I2C3, p.PH7, p.PH8, Hertz(50_000), Default::default());

    let delay_ref: &'static mut Delay = unsafe { &mut DELAY };
    let touch = ft5336::Ft5336::new(&i2c, 0x38, delay_ref).unwrap();
    spawner.spawn(catch_touch(touch, i2c)).unwrap();
    let led = Output::new(p.PI1, Level::High, Speed::Low);

    let mut D0 = Output::new(p.PC7, Level::Low, Speed::Low);
    let mut D1 = Output::new(p.PC6, Level::Low, Speed::Low);
    let mut D2 = Output::new(p.PG6, Level::Low, Speed::Low);
    let mut D3 = Output::new(p.PB4, Level::Low, Speed::Low);
  

    spawner.spawn(buttons_task(D0, D1, D2, D3)).unwrap();

    loop {
        Timer::after_millis(1000).await;
    }
}
*/
#![no_std]
#![no_main]


use embassy_executor::Spawner;

use defmt::*;

use {defmt_rtt as _, panic_probe as _};

// Support functions
mod sdram;
mod rcc;
mod display;
use embassy_stm32::ltdc::Ltdc;

extern crate alloc;
use embedded_alloc::TlsfHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {

    let mut config = rcc::init_rcc();

    let p = embassy_stm32::init(config);
    // Unfocatnly I didint find other way to do this more compact
    let sdram_pins = sdram::SdramPins {
        a0: p.PF0,
        a1: p.PF1,
        a2: p.PF2,
        a3: p.PF3,
        a4: p.PF4,
        a5: p.PF5,
        a6: p.PF12,
        a7: p.PF13,
        a8: p.PF14,
        a9: p.PF15,
        a10: p.PG0,
        a11: p.PG1,
        ba0: p.PG4,
        ba1: p.PG5,

        d0: p.PD14,
        d1: p.PD15,
        d2: p.PD0,
        d3: p.PD1,
        d4: p.PE7,
        d5: p.PE8,
        d6: p.PE9,
        d7: p.PE10,
        d8: p.PE11,
        d9: p.PE12,
        d10: p.PE13,
        d11: p.PE14,
        d12: p.PE15,
        d13: p.PD8,
        d14: p.PD9,
        d15: p.PD10,

        nbl0: p.PE0,
        nbl1: p.PE1,

        sdclk: p.PG8,
        sdnwe: p.PG15,
        sdcke0: p.PH3,
        sdnc: p.PC3,
        sdne0: p.PH5,
        sdnras: p.PF11,
    };

    let mut sdram = sdram::init_sdram(p.FMC, sdram_pins);
    // Configure MPU for external SDRAM (64 Mbit = 8 Mbyte)
    // MPU is disabled by default
    const SDRAM_SIZE: usize = 8 * 1024 * 1024;
    let mut delay = embassy_time::Delay;

    unsafe {
        // Initialise controller and SDRAM
        let ram_ptr: *mut u32 = sdram.init(&mut delay) as *mut _;

        info!("SDRAM Initialized at {:x}", ram_ptr as usize);

        // Convert raw pointer to slice
        HEAP.init(ram_ptr as usize, SDRAM_SIZE)
    };

    let pins = display::DisplayPins {
        r0: p.PI15,
        r1: p.PJ0,
        r2: p.PJ1,
        r3: p.PJ2,
        r4: p.PJ3,
        r5: p.PJ4,
        r6: p.PJ5,
        r7: p.PJ6,

        g0: p.PJ7,
        g1: p.PJ8,
        g2: p.PJ9,
        g3: p.PJ10,
        g4: p.PJ11,
        g5: p.PK0,
        g6: p.PK1,
        g7: p.PK2,

        b0: p.PE4,
        b1: p.PJ13,
        b2: p.PJ14,
        b3: p.PJ15,
        b4: p.PG12,
        b5: p.PK4,
        b6: p.PK5,
        b7: p.PK6,

        hsync: p.PI10,
        vsync: p.PI9,
        clk: p.PI14,
        de: p.PK7,

        lcd_en: p.PI12,
        backlight: p.PK3,
    };

    let ltdc = p.LTDC;
    display::init_display(pins, ltdc);
    info!("Init LTDC display");
    ltdc.enable();

}
