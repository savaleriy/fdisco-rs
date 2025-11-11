//! UART example using Embassy async DMA on STM32F7 (USART6)
//!
//! This example demonstrates how to set up and use USART6 on the STM32F746NG microcontroller
//! with asynchronous DMA-driven UART transmission using the Embassy framework.
//!
//! Features:
//! - Configures USART6 peripheral with default UART settings (115200 baud, 8N1)
//! - Uses DMA channels for TX and RX to offload data transfers from the CPU
//! - Sends incrementing "Hello DMA World {n}!" messages repeatedly with DMA write calls
//! - Uses `heapless::String` and `core::fmt::Write` to build formatted message buffers safely
//! - Employs `defmt` for logging messages during runtime
//! - Interrupts bound for USART6 to enable async DMA operation
//!
//! Pinout (STM32F746NG):
//! - PC6: USART6_TX (transmit)
//! - PC7: USART6_RX (receive)
//!
//! DMA channels used:
//! - DMA2 Channel 6: USART6_TX
//! - DMA2 Channel 1: USART6_RX
//!
//! This example is a good starting point for UART communication in no_std Rust
//! applications using Embassy on STM32 MCUs, illustrating async writes with DMA.
//!
//! Requires:
//! - STM32F7 HAL with Embassy support
//! - Defmt for logging
//! - Panic probe for debugging panics
//!
//! To run, connect a serial terminal to USART6 pins configured at 115200 baud,
//! 8 data bits, no parity, 1 stop bit.
//! More info about module https://docs.embassy.dev/embassy-stm32/git/stm32f746ng/usart/index.html

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::usart::{BufferedUart, Config};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embedded_io_async::{Read, Write};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART6 => usart::BufferedInterruptHandler<peripherals::USART6>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let mut config = Config::default();

    let mut tx_buf = [0u8; 256];
    let mut rx_buf = [0u8; 256];
    config.baudrate = 9600;

    #[rustfmt::skip]
    let mut usart = BufferedUart::new(
        p.USART6,
        p.PC7,      // RX pin
        p.PC6,      // TX pin
        &mut tx_buf,
        &mut rx_buf,
        Irqs,       // Interrupt binding
        config,
    )
    .expect("Failed to initialize USART6");

    usart.write_all(b"Hello World\r\n").await.unwrap();
    info!("wrote Hello, starting echo");

    let mut buf = [0; 64];
    let mut pos = 0;

    loop {
        // Read one byte at a time
        let mut byte = [0u8; 1];
        usart.read_exact(&mut byte).await.unwrap();

        buf[pos] = byte[0];
        pos += 1;

        // Check for newline to detect end of message
        if byte[0] == b'\n' || pos >= buf.len() {
            info!("Read complete message: {} bytes", pos);
            info!(
                "Content: {}",
                core::str::from_utf8(&buf[..pos]).unwrap_or("<non-utf8>")
            );

            usart.write_all(&buf[..pos]).await.unwrap();
            info!("Echoed back {} bytes", pos);

            pos = 0; // Reset for next message
        }
    }
}
