#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

const GREET_HEADER: [u8; 4] = [114, 5, 1, 4];

const YELL_BACK_HEADER: [u8; 4] = [19, 19, 8, 10];

use embassy_executor::Executor;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Ticker, Timer};
use embedded_hal_async::digital::Wait;
use esp_backtrace as _;
use esp_wifi::esp_now::{EspNow, PeerInfo, BROADCAST_ADDRESS};
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    gpio::{Gpio0, Input, Output, PullDown, PushPull},
    interrupt,
    peripherals::{self, Peripherals},
    prelude::*,
    systimer::SystemTimer,
    timer::TimerGroup,
    Rng, Rtc, IO,
};
use log::*;
use static_cell::StaticCell;

static EXECUTOR: StaticCell<embassy_executor::Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    let syst = SystemTimer::new(peripherals.SYSTIMER);
    info!("Peripherals initialized.");

    esp_wifi::init_heap();

    let mut rng = Rng::new(peripherals.RNG);
    #[cfg(any(feature = "master"))]
    let token = rng.random();

    esp_wifi::initialize(syst.alarm0, rng, &clocks).unwrap();
    let esp_now = esp_wifi::esp_now::esp_now().initialize().unwrap();

    info!("ESPNow initialized.");

    embassy::init(&clocks, timer_group0.timer0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    interrupt::enable(peripherals::Interrupt::GPIO, interrupt::Priority::Priority1).unwrap();

    let executor = EXECUTOR.init(Executor::new());

    #[cfg(any(feature = "master"))]
    let pin = io.pins.gpio0.into_pull_down_input();

    #[cfg(not(any(feature = "master")))]
    let pin = io.pins.gpio0.into_push_pull_output();

    executor.run(|spawner| {
        #[cfg(any(feature = "master"))]
        spawner.spawn(master_task(esp_now, token, pin)).ok();

        #[cfg(not(any(feature = "master")))]
        spawner.spawn(slave_task(esp_now, pin)).ok();
    });
}

#[embassy_executor::task]
async fn master_task(mut espnow: EspNow, token: u32, mut pin: Gpio0<Input<PullDown>>) {
    let mut peer = None;
    let mut ticker = Ticker::every(Duration::from_millis(200));
    let mut counter = 0u8;
    let mut greeting = [0u8; 8];
    greeting[..4].copy_from_slice(&GREET_HEADER);
    let token: [u8; 4] = [
        ((token >> 24) & 0xff) as u8,
        ((token >> 16) & 0xff) as u8,
        ((token >> 8) & 0xff) as u8,
        (token & 0xff) as u8,
    ];
    greeting[4..].copy_from_slice(&token);
    loop {
        if peer.is_none() {
            debug!("Still no peer.");
            espnow.send(&BROADCAST_ADDRESS, &greeting).unwrap();
            debug!("Greet message sent.");
            let res = select(ticker.next(), espnow.receive_async()).await;
            if let Either::Second(msg) = res {
                debug!("Message received: {:?}", msg.get_data());
                if msg.get_data()[4..8].as_ref() == &token
                    && msg.get_data()[0..4].as_ref() == &YELL_BACK_HEADER
                {
                    if !espnow.peer_exists(&msg.info.src_address).unwrap() {
                        espnow
                            .add_peer(PeerInfo {
                                peer_address: msg.info.src_address,
                                lmk: None,
                                channel: None,
                                encrypt: false,
                            })
                            .unwrap();
                    }
                    info!("Peer added.");
                    peer = Some(msg.info.src_address);
                }
            }
            Timer::after(Duration::from_millis(200)).await;
        } else {
            loop {
                let start = Instant::now();
                espnow.send(&peer.unwrap(), &[counter]).unwrap();
                pin.wait_for_rising_edge().await.unwrap();
                let end = Instant::now();
                pin.wait_for_falling_edge().await.unwrap();
                let round_trip = end - start;
                // display round trip time
                info!("Round trip time: {:?}", round_trip);
                counter += 1;
                Timer::after(Duration::from_millis(200)).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn slave_task(mut espnow: EspNow, mut pin: Gpio0<Output<PushPull>>) {
    pin.set_low().unwrap();
    info!("Waiting for greeting.");
    let peer;
    loop {
        let msg = espnow.receive_async().await;
        debug!("Message received: {:?}", msg.get_data());
        if msg.get_data().as_ref()[..4] == GREET_HEADER {
            if !espnow.peer_exists(&msg.info.src_address).unwrap() {
                espnow
                    .add_peer(PeerInfo {
                        peer_address: msg.info.src_address,
                        lmk: None,
                        channel: None,
                        encrypt: false,
                    })
                    .unwrap();
            }
            info!("Peer added.");
            peer = msg.info.src_address;
            let mut yell_back = [0u8; 8];
            yell_back[0..4].copy_from_slice(&YELL_BACK_HEADER);
            yell_back[4..8].copy_from_slice(msg.get_data()[4..8].as_ref());
            espnow.send(&peer, &yell_back).unwrap();
            debug!("Yell back message sent.");
            break;
        }
    }
    loop {
        let msg = espnow.receive_async().await;
        pin.set_high().unwrap();
        info!("Counter message received: {:?}", msg.get_data());
        Timer::after(Duration::from_millis(200)).await;
        pin.set_low().unwrap();
    }
}
