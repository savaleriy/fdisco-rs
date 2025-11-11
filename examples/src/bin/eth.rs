#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::
{
    tcp::TcpSocket,
    {
        Ipv4Address, StackResources
    },
    Ipv4Cidr,

};

use embassy_stm32::{
    rcc::{
        AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv,
        PllRDiv, PllSource, Sysclk,
    },
    time::mhz,
    {
        bind_interrupts, eth, peripherals, rng, Config
    },
    rng::Rng,
    peripherals::ETH,
    eth::{Ethernet, GenericPhy, PacketQueue},
};

use embassy_time::Timer;
use embedded_io_async::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use heapless::Vec;

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    RNG => rng::InterruptHandler<peripherals::RNG>;
});

type Device = Ethernet<'static, ETH, GenericPhy>;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device>) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();

    // Enable external 25 MHz oscillator
    config.rcc.hse = Some(Hse {
        freq: mhz(25),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll_src = PllSource::HSE;

    // PLL1 for SYSCLK = 400 / 2 = 200 MHz
    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV25,  // PLLM
        mul: PllMul::MUL400,       // PLLN
        divp: Some(PllPDiv::DIV2), // PLLP -> SYSCLK
        divq: Some(PllQDiv::DIV9), // PLLQ
        divr: None,
    });

    // Set SYSCLK source to PLL1_P output
    config.rcc.sys = Sysclk::PLL1_P;

    // Optional: Set bus dividers to match peripheral limits
    config.rcc.ahb_pre = AHBPrescaler::DIV1; // 200 MHz
    config.rcc.apb1_pre = APBPrescaler::DIV4; // Max 50 MHz
    config.rcc.apb2_pre = APBPrescaler::DIV2; // Max 100 MHz

    // Optional: PLLSAI and PLLI2S setup
    config.rcc.pllsai = Some(Pll {
        prediv: PllPreDiv::DIV25,
        mul: PllMul::MUL384,
        divp: Some(PllPDiv::DIV8),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV5),
    });

    config.rcc.plli2s = Some(Pll {
        prediv: PllPreDiv::DIV25,
        mul: PllMul::MUL100,
        divp: Some(PllPDiv::DIV2),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV2),
    });

    let p = embassy_stm32::init(config);

    // Generate random seed.
    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0; 8];
    rng.fill_bytes(&mut seed);
    let seed = u64::from_le_bytes(seed);


    //let mac_addr = [0x02, 0x00, 0x12, 0x34, 0x56, 0x78];
    let mac_addr = [0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];

    static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();

    let device = Ethernet::new(
        PACKETS.init(PacketQueue::<4, 4>::new()),
        p.ETH,
        Irqs,
        p.PA1,
        p.PA2,
        p.PC1,
        p.PA7,
        p.PC4,
        p.PC5,
        p.PG13,
        p.PB13,
        p.PG11,
        GenericPhy::new_auto(),
        mac_addr,
    );
    
    // Print info about device
    // HOW TO GET INFO ABOUT CONNECTED DEVICE ??
    
    // let config = embassy_net::Config::dhcpv4(Default::default());
    // Replace the DHCP config with static IP
    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 210, 201), 24),
        dns_servers: Vec::new(),
        gateway: Some(Ipv4Address::new(192, 168, 210, 1)), // Some GateWay
    });

    // Init network stack
    static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();
    let (stack, runner) =
        embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);

    // Launch network task
    spawner.spawn(unwrap!(net_task(runner)));

    // Ensure DHCP configuration is up before trying connect
    stack.wait_config_up().await;

    info!("Network task initialized");

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    info!("Network stack initialized. Attempting to connect...");

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
        let remote_endpoint = (Ipv4Address::new(192, 168, 210, 97), 1234); 
        //let remote_endpoint = (Ipv4Address::new(192, 168, 210, 1), 8000);
        info!("Trying to connect to {:?}...", remote_endpoint);

        match socket.connect(remote_endpoint).await {
            Ok(()) => {
                info!("✅ Connected successfully!");
                // Proceed with your communication
                loop {
                    match socket.write_all(b"Hello\n").await {
                        Ok(()) => {
                            info!("Message sent.");
                        },
                        Err(e) => {
                            info!("❌ Write error (connection lost?): {:?}", e);
                            break; // Break inner loop to retry connection
                        }
                    }
                    Timer::after_secs(1).await;
                }
            },
            Err(e) => {
                // This error likely indicates no physical link or no server at the endpoint.
                info!("Connection failed: {:?}", e);
                Timer::after_secs(5).await; // Wait a bit longer before retrying
            }
        }
    }
    /*    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
        let remote_endpoint = (Ipv4Address::new(192, 168, 210, 1), 8000);
        info!("connecting...");
        let r = socket.connect(remote_endpoint).await;
        if let Err(e) = r {
            info!("connect error: {:?}", e);
            Timer::after_secs(1).await;
            continue;
        }
        info!("connected!");
        loop {
            let r = socket.write_all(b"Hello\n").await;
            if let Err(e) = r {
                info!("write error: {:?}", e);
                break;
            }
            Timer::after_secs(1).await;
        }
    }*/
}
