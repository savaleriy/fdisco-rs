#![no_std]
#![no_main]

use defmt::{panic, *};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_stm32::{
    bind_interrupts, peripherals,
    rcc::{
        AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv,
        PllRDiv, PllSource, Sysclk,
    },
    time::mhz,
    usb,
    usb::{Driver, Instance},
    Config,
};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
    Builder,
};

use {defmt_rtt as _, panic_probe as _};

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn echo<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &buf[..n];
        info!("data: {:x} -> 0x{:02x}", data, data);

        let message = core::str::from_utf8(data).unwrap_or("<invalid UTF-8>");
        info!("text : {}", message);

        class.write_packet(data).await?;
    }
}
bind_interrupts!(struct Irqs {
    OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
});

// If you are trying this and your USB device doesn't connect, the most
// common issues are the RCC config and vbus_detection
//
// See https://embassy.dev/book/#_the_usb_examples_are_not_working_on_my_board_is_there_anything_else_i_need_to_configure
// for more information.
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello World!");

    let mut config = Config::default();

    // Enable external 25 MHz oscillator
    config.rcc.hse = Some(Hse {
        freq: mhz(25),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll_src = PllSource::HSE;

    // PLL1 for SYSCLK = 400 / 2 = 200 MHz
    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV25,  // PLLM = 25
        mul: PllMul::MUL384,       // PLLN = 384 (25 MHz / 25 * 384 = 384 MHz)
        divp: Some(PllPDiv::DIV4), // PLLP = 4 -> SYSCLK = 384 / 4 = 96 MHz
        divq: Some(PllQDiv::DIV8), // PLLQ = 8 -> USB clock = 384 / 8 = 48 MHz
        divr: None,
    });

    // Set SYSCLK source to PLL1_P output
    config.rcc.sys = Sysclk::PLL1_P;

    // Adjust bus dividers for the new SYSCLK frequency
    config.rcc.ahb_pre = AHBPrescaler::DIV1; // 96 MHz
    config.rcc.apb1_pre = APBPrescaler::DIV2; // 48 MHz (Max 50 MHz)
    config.rcc.apb2_pre = APBPrescaler::DIV1; // 96 MHz (Max 100 MHz)

    // Optional: Adjust PLLSAI and PLLI2S if needed for other peripherals
    config.rcc.pllsai = Some(Pll {
        prediv: PllPreDiv::DIV25,
        mul: PllMul::MUL192,
        divp: Some(PllPDiv::DIV4),
        divq: Some(PllQDiv::DIV4),
        divr: Some(PllRDiv::DIV2),
    });

    config.rcc.plli2s = Some(Pll {
        prediv: PllPreDiv::DIV25,
        mul: PllMul::MUL100,
        divp: Some(PllPDiv::DIV2),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV2),
    });

    let p = embassy_stm32::init(config);
    // Create the driver, from the HAL.
    let mut ep_out_buffer = [0u8; 256];
    let mut config = embassy_stm32::usb::Config::default();

    // Do not enable vbus_detection. This is a safe default that works in all boards.
    // However, if your USB device is self-powered (can stay powered on if USB is unplugged), you need
    // to enable vbus_detection to comply with the USB spec. If you enable it, the board
    // has to support it or USB won't work at all. See docs on `vbus_detection` for details.
    config.vbus_detection = false;

    let driver = Driver::new_fs(
        p.USB_OTG_FS,
        Irqs,
        p.PA12,
        p.PA11,
        &mut ep_out_buffer,
        config,
    );
    // Create embassy-usb Config
    // In lsusb we should see something like this
    // > Bus 001 Device 122: ID c0de:cafe Embassy USB-serial example
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    // Create classes on the builder.
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    // Do stuff with the class!
    let echo_fut = async {
        loop {
            class.wait_connection().await;
            info!("Connected");
            let _ = echo(&mut class).await;
            info!("Disconnected");
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, echo_fut).await;
}
