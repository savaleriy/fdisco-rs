#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART6 => usart::InterruptHandler<peripherals::USART6>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting UART echo example");

    let p = embassy_stm32::init(Default::default());
    let config = Config::default();

    #[rustfmt::skip]
    let mut usart = Uart::new(
        p.USART6,
        p.PC7,      // RX
        p.PC6,      // TX
        Irqs,
        p.DMA2_CH6,
        p.DMA2_CH1,
        config,
    )
    .expect("Failed to initialize USART6");

    // Buffer to read into
    let mut buf = [0u8; 16];

    loop {
        // Read data into buffer
        usart.read(&mut buf).await.expect("UART read failed");
        info!("Read");

        // Echo back exactly the read buffer
        usart.write(&buf).await.expect("UART write failed");
        info!("Write");
    }
}
