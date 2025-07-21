//! To run, connect a serial terminal to USART6 pins configured at 115200 baud,
//! 8 data bits, no parity, 1 stop bit.
//! More info about module https://docs.embassy.dev/embassy-stm32/git/stm32f746ng/usart/index.html

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::fmt::Write;
use defmt::*;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use heapless::String;

use embassy_executor::Spawner;
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

//Declate a channel of 2 u32s
static SHARED: Channel<ThreadModeRawMutex, u32, 2> = Channel::new();

#[embassy_executor::task]
async fn async_task_one() {
    loop {
        SHARED.send(1).await;
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
async fn async_task_two() {
    loop {
        SHARED.send(2).await;
        Timer::after(Duration::from_millis(1000)).await;
    }
}

bind_interrupts!(struct Irqs {
    USART6 => usart::InterruptHandler<peripherals::USART6>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize and create handle for device peripherals
    let p = embassy_stm32::init(Default::default());

    //Configure UART
    #[rustfmt::skip]
    let mut usart = Uart::new(
        p.USART6,
        p.PC7,      // RX
        p.PC6,      // TX
        Irqs,
        p.DMA2_CH6,
        p.DMA2_CH1,
        Config::default(),
    )
    .expect("Failed to initialize USART6");

    // Create empty String for message
    let mut msg: String<16> = String::new();
    let mut buf = [0u8; 16];

    // Spawn async blinking task
    spawner.spawn(async_task_one()).unwrap();
    spawner.spawn(async_task_two()).unwrap();

    loop {
        // Handle channel messages
        if let Ok(val) = SHARED.try_receive() {
            msg.clear();
            core::writeln!(&mut msg, "Received: {:02}\r\n", val).unwrap();
            usart.write(msg.as_bytes()).await.unwrap();
        }

        // Handle UART input
        if let Ok(_) = usart.read(&mut buf).await {
            // Echo back what we received
            usart.write(&buf).await.unwrap();

            // Also log via defmt
            if buf[0] == b'\r' {
                info!("Received carriage return");
            } else {
                info!("Received: {}", buf[0] as char);
            }
        }
    }
}
