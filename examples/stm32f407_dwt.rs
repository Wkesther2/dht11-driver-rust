#![no_main]
#![no_std]

use dht11_driver_rust::DHT11;
use stm32f4xx_hal::{gpio::{GpioExt, PinState}, pac, prelude::*, rcc};
use embedded_hal::delay::DelayNs;

#[cfg(feature = "use-defmt")]
use defmt_rtt as _;
#[cfg(feature = "use-defmt")]
use panic_probe as _;
#[cfg(not(feature = "use-defmt"))]
use panic_halt as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    #[cfg(not(feature = "use-dwt"))]
        compile_error!(
        "This example is designed for software-timing only. \
        Please build it using: cargo run --example f407_no_dwt --no-default-features"
    );
    // Take the Core Peripherals
    let mut cp = cortex_m::Peripherals::take().unwrap();
    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    // Take the MCU-specific Peripherals (GPIO, RCC, TIMERS)
    let dp = pac::Peripherals::take().unwrap();

    // Clock Configuration for STM32F407 (running at 168 MHz)
    let config = rcc::Config::default()
        .sysclk(168.MHz());

    // Freeze the RCC (Reset and Clock Control) configuration
    let mut rcc = dp.RCC.freeze(config);

    // Initialize GPIOA Pin 0 for the DHT11
    // We use Open Drain to allow the sensor to pull the line LOW
    let gpioa = dp.GPIOA.split(&mut rcc);
    let dht11_pin = gpioa.pa0.into_open_drain_output_in_state(PinState::High);

    // Initialize Timer 6 for microsecond delays
    let mut delay = dp.TIM6.delay_us(&mut rcc);
    
    // Create and initialize the DHT11 driver
    // The Typestate pattern ensures we can't read before initialization!
    let dht11 = DHT11::new(dht11_pin, rcc.clocks.sysclk());
    let mut dht11 = dht11.initialize(&mut delay).unwrap();

    // Alternatively we can use the new_and_init method to create and initialize the Sensor in one Step
    // let mut dht11 = DHT11::new_and_initialized(dht11_pin, &mut delay, rcc.clocks.sysclk()).unwrap();

    // Wait for 2 Seconds before reading the Sensor
    delay.delay_ms(2_000);

    loop {
        // Request new data from the sensor
        match dht11.read_temp_and_hum(&mut delay) {
            Ok(_measurement) => {
                #[cfg(feature = "use-defmt")]
                {
                    // --- OPTION 1: Floating Point (f32) ---
                    // Best for: Human-readable logs and MCUs with an FPU (like STM32F4).
                    // ⚠️ Requires software emulation on MCUs without FPU (increases binary size).
                    defmt::println!("Temp: {} °C, Hum: {} % (f32)", 
                        _measurement.temp_as_f32(), 
                        _measurement.hum_as_f32()
                    );
    
                    /* // --- OPTION 2: Raw Integral/Decimal ---
                    // Best for: Exact bit-representation and debugging the sensor handshake.
                    // Fast and 0% overhead.
                    defmt::println!("Temperature: {}.{} °C, Humidity: {}.{} % (Raw)",
                        _measurement.temperature.integral, 
                        _measurement.temperature.decimal,
                        _measurement.humidity.integral, 
                        _measurement.humidity.decimal
                    );
                    */
    
                    /*
                    // --- OPTION 3: Fixed Point (i16) ---
                    // Best for: Control loops and MCUs without FPU. 
                    // 255 means 25.5°C. Very fast and memory efficient.
                    defmt::println!("Temp (fixed): {}, Hum (fixed): {} (1/10th units)", 
                        _measurement.temp_as_fixed(), 
                        _measurement.hum_as_fixed()
                    );
                    */
                }
                #[cfg(not(feature = "use-defmt"))]
                {
                    // Here you can use any alternative to defmt
                }
            },
            Err(_e) => {
                // ⚠️ Handle errors (like ChecksumMismatch or Timeout)
                #[cfg(feature = "use-defmt")]
                defmt::error!("Error reading DHT11: {}", _e);

                #[cfg(not(feature = "use-defmt"))]
                {
                    // Here you can use any alternative to defmt
                }
            }
        }

        // Wait 2 seconds before the next measurement
        // DHT11 needs time to settle between readings!
        delay.delay_ms(2000);
    }
}