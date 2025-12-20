#![no_main]
#![no_std]

use dht11_driver_rust::DHT11;
use stm32f4xx_hal::{gpio::{GpioExt, PinState}, pac, prelude::*, rcc};
use embedded_hal::delay::DelayNs;
use defmt_rtt as _;
use panic_probe as _;

#[cortex_m_rt::entry]
fn main() -> ! {    
    // Take the Core Peripherals and start DWT Cycle Counter only if the "dwt" feature is enabled
    #[cfg(feature = "dwt")]
    {
        let mut cp = cortex_m::Peripherals::take().unwrap();
        cp.DCB.enable_trace();
        cp.DWT.enable_cycle_counter();
    }

    // Take the MCU-specific Peripherals (GPIO, RCC, TIMERS)
    let dp = pac::Peripherals::take().unwrap();

    // Clock Configuration for STM32F407 (running at 168 MHz)
    let config = rcc::Config::default()
        .sysclk(168.MHz())
        .pclk1(42.MHz())
        .pclk2(84.MHz());

    // ❄️ Freeze the RCC (Reset and Clock Control) configuration
    let mut rcc = dp.RCC.freeze(config);

    // 🔌 Initialize GPIOA Pin 0 for the DHT11
    // We use Open Drain to allow the sensor to pull the line LOW 📉
    let gpioa = dp.GPIOA.split(&mut rcc);
    let dht11_pin = gpioa.pa0.into_open_drain_output_in_state(PinState::High);

    // ⏱️ Initialize Timer 6 for microsecond delays
    let mut delay = dp.TIM6.delay_us(&mut rcc);
    
    // 🌡️ Create and initialize the DHT11 driver
    // The Typestate pattern ensures we can't read before initialization! 🛡️
    let dht11 = DHT11::new(dht11_pin);
    let mut dht11 = dht11.initialize(&mut delay).unwrap();

    loop {
        // Request new data from the sensor
        match dht11.read_temp_and_hum(&mut delay) {
            Ok(measurement) => {
                // 📊 Print the results to the console via RTT
                defmt::println!("Humidity: {}.{} % 💧",
                    measurement.humidity.integral, 
                    measurement.humidity.decimal
                );
                defmt::println!("Temperature: {}.{} °C 🌡️",
                    measurement.temperature.integral, 
                    measurement.temperature.decimal
                );
            },
            Err(e) => {
                // ⚠️ Handle errors (like ChecksumMismatch or Timeout)
                defmt::error!("Error reading DHT11: {}", e);
            }
        }

        // 💤 Wait 2 seconds before the next measurement
        // DHT11 needs time to settle between readings! ⏳
        delay.delay_ms(2000);
    }
}