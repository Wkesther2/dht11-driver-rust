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

    // Alternatively we can use the new_and_init method to create and initialize the Sensor in one Step
    // let mut dht11 = DHT11::new_and_initialized(dht11_pin, &mut delay).unwrap();

    // Wait for 2 Seconds before reading the Sensor
    delay.delay_ms(2_000);

    loop {
        // Request new data from the sensor
        match dht11.read_temp_and_hum(&mut delay) {
            Ok(measurement) => {
                // --- OPTION 1: Floating Point (f32) ---
                // Best for: Human-readable logs and MCUs with an FPU (like STM32F4).
                // ⚠️ Requires software emulation on MCUs without FPU (increases binary size).
                defmt::println!("Temp: {} °C, Hum: {} % (f32)", 
                    measurement.temp_as_f32(), 
                    measurement.hum_as_f32()
                );

                /* // --- OPTION 2: Raw Integral/Decimal ---
                // Best for: Exact bit-representation and debugging the sensor handshake.
                // Fast and 0% overhead.
                defmt::println!("Temperature: {}.{} °C, Humidity: {}.{} % (Raw)",
                    measurement.temperature.integral, 
                    measurement.temperature.decimal,
                    measurement.humidity.integral, 
                    measurement.humidity.decimal
                );
                */

                /*
                // --- OPTION 3: Fixed Point (i16) ---
                // Best for: Control loops and MCUs without FPU. 
                // 255 means 25.5°C. Very fast and memory efficient.
                defmt::println!("Temp (fixed): {}, Hum (fixed): {} (1/10th units)", 
                    measurement.temp_as_fixed(), 
                    measurement.hum_as_fixed()
                );
                */
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