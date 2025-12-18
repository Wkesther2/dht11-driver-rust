#![no_main]
#![no_std]

use cortex_m::asm::nop;
use dht11_driver_rust::DHT11;
use stm32f4xx_hal::{self as _, gpio::GpioExt, rcc::RccExt, timer::TimerExt};
use defmt_rtt as _;
use panic_probe as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    // Take the MCUs Peripherals
    let dp = stm32f4xx_hal::pac::Peripherals::take().unwrap();

    // Take the RCC Peripheral and configure the System Clock
    let mut rcc = dp.RCC.constrain();

    // Initialize the GPIO Pin for the DHT11
    let gpioa = dp.GPIOA.split(&mut rcc);
    let dht11_pin = gpioa.pa0.into_open_drain_output();

    // Initialize Timer
    let mut delay = dp.TIM6.delay_us(&mut rcc);
    
    // Initialize DHT11
    let dht11 = DHT11::new(dht11_pin);
    match dht11.initialize(&mut delay) {
        Ok(_) => defmt::info!("DHT11 antwortet"),
        Err(e) => defmt::error!("Error: {:?}", e),
    }

    dht11_driver_rust::exit()
}
