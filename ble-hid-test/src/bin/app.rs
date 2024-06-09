#![no_std]
#![no_main]

use esp32c3_hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Delay};
use esp_backtrace as _;
use esp_println::println;

use esp_wifi::{initialize, EspWifiInitFor};

use esp32c3_hal::{systimer::SystemTimer, Rng};
#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    // setup logger
    // To change the log_level change the env section in .cargo/config.toml
    // or remove it and set ESP_LOGLEVEL manually before running cargo run
    // this requires a clean rebuild because of https://github.com/rust-lang/cargo/issues/10358
    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");
    log::info!("Hi there");
    println!("Hello world!");
    let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let _init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();
    loop {
        println!("Loop...");
        delay.delay_ms(500u32);
    }
}
