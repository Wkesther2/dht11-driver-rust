#![doc = include_str!("../README.md")]
#![deny(unsafe_code)]
#![cfg_attr(not(test), no_std)]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use core::marker::PhantomData;

#[cfg(feature = "use-dwt")]
use fugit::HertzU32;

#[cfg(feature = "use-dwt")]
use cortex_m::peripheral::DWT;

use crate::error::DHT11Error;

/// Max time to wait for a state change before giving up
#[allow(unused)]
const TIMEOUT_US: u32 = 200;

#[cfg(not(feature = "use-dwt"))]
#[allow(unused)]
const TIMEOUT_CYCLES: u32 = 2_000_000;

pub mod error {
    pub type Result<T, E> = core::result::Result<T, DHT11Error<E>>;

    #[derive(Debug, Clone, Copy)]
    #[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
    pub enum DHT11Error<E> {
        NoResponse,
        ChecksumMismatch,
        PinError(E),
        Timeout,
    }
}

// --- Typestate Pattern for Safety ---
pub struct Idle;
pub struct Uninitialized;
pub struct Initialized;

pub struct SensorValue {
    pub integral: u8,
    pub decimal: u8,
}

pub type Temperature = SensorValue;
pub type Humidity = SensorValue;

pub struct Measurement {
    pub humidity: Humidity,
    pub temperature: Temperature,
    pub checksum: u8,
}

impl Measurement {
    pub fn temp_as_f32(&self) -> f32 {
        self.temperature.integral as f32 + (self.temperature.decimal as f32 / 10.0)
    }

    pub fn temp_as_fixed(&self) -> i16 {
        (self.temperature.integral as i16 * 10) + (self.temperature.decimal as i16)
    }

    pub fn hum_as_f32(&self) -> f32 {
        self.humidity.integral as f32 + (self.humidity.decimal as f32 / 10.0)
    }

    pub fn hum_as_fixed(&self) -> i16 {
        (self.humidity.integral as i16 * 10) + (self.humidity.decimal as i16)
    }
}

pub struct DHT11<PIN, STATE> {
    pin: PIN,
    _state: PhantomData<STATE>,
    clock_info: u32,
}

impl<PIN, E, STATE> DHT11<PIN, STATE>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    fn is_bit_high(&self, _low_duration: u32, high_duration: u32) -> bool {
        #[cfg(feature = "use-dwt")]
        {
            let cycles_per_us = self.clock_info / 1_000_000;
            high_duration > (40 * cycles_per_us)
        }

        #[cfg(not(feature = "use-dwt"))]
        {
            high_duration > _low_duration
        }
    }

    /// Waits for the pin to reach a certain state and measures the time taken
    fn wait_for_state(&mut self, target_high: bool, _delay: &mut impl DelayNs) -> error::Result<u32, E> {
        #[cfg(feature = "use-dwt")]
        {
            let cycles_per_us = self.clock_info / 1_000_000;
            let start = DWT::cycle_count();
            // Loop until the pin state matches our target
            while self.pin.is_low().map_err(DHT11Error::PinError)? == target_high {
                if DWT::cycle_count().wrapping_sub(start) > (TIMEOUT_US * cycles_per_us) {
                    return Err(DHT11Error::Timeout);
                }
            }
            Ok(DWT::cycle_count().wrapping_sub(start))
        }

        #[cfg(not(feature = "use-dwt"))]
        {
            let mut count: u32 = 0;
            while self.pin.is_low().map_err(DHT11Error::PinError)? == target_high {
                count += 1;
                if count > TIMEOUT_CYCLES { return Err(DHT11Error::Timeout); }
            }
            Ok(count)
        }
    }

    /// Sends the 18ms pulse to wake up the sensor
    fn send_start_signal(&mut self, delay: &mut impl DelayNs) -> error::Result<(), E> {
        self.pin.set_high().map_err(DHT11Error::PinError)?;
        delay.delay_ms(1);

        // Pull low for 18-20ms to signal the start
        self.pin.set_low().map_err(DHT11Error::PinError)?;
        delay.delay_ms(20);
        
        // Release the line and wait for the sensor to pull it low
        self.pin.set_high().map_err(DHT11Error::PinError)?;

        // Wait for the sensor's acknowledgment (LOW)
        self.wait_for_state(false, delay)?;

        Ok(())
    }
}

impl<PIN, E> DHT11<PIN, Idle>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    #[cfg(feature = "use-dwt")]
    pub fn new(pin: PIN, clock_info: HertzU32) -> DHT11<PIN, Uninitialized> {
        DHT11 {
            pin,
            _state: PhantomData,
            clock_info: clock_info.to_Hz(),
        }
    }

    #[cfg(not(feature = "use-dwt"))]
    pub fn new(pin: PIN) -> DHT11<PIN, Uninitialized> {
        DHT11 {
            pin,
            _state: PhantomData,
            clock_info: 0,
        }
    }

    /// Creates and initializes the sensor in one go.
    /// ⚠️ WARNING: This is a blocking call that takes ~3 seconds to complete.
    #[cfg(feature = "use-dwt")]
    pub fn new_and_initialized(pin: PIN, delay: &mut impl DelayNs, clock_info: HertzU32) -> error::Result<DHT11<PIN, Initialized>, E> {
        let uninit = Self::new(pin, clock_info);

        let init = uninit.initialize(delay)?;

        delay.delay_ms(2_000);

        Ok(init)
    }

    /// Creates and initializes the sensor in one go.
    /// ⚠️ WARNING: This is a blocking call that takes ~3 seconds to complete.
    #[cfg(not(feature = "use-dwt"))]
    pub fn new_and_initialized(pin: PIN, delay: &mut impl DelayNs) -> error::Result<DHT11<PIN, Initialized>, E> {
        let uninit = Self::new(pin);

        let init = uninit.initialize(delay)?;

        delay.delay_ms(2_000);

        Ok(init)
    }
}

impl<PIN, E> DHT11<PIN, Uninitialized>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    /// Initializes the sensor with required power-on delays
    /// ⚠️ WARNING: Wait at least 2 seconds after initialization before reading the sensor
    pub fn initialize(mut self, delay: &mut impl DelayNs) -> error::Result<DHT11<PIN, Initialized>, E> {
        // Perform a test handshake to ensure the sensor is present
        self.send_start_signal(delay)?;

        Ok(DHT11 {
            pin: self.pin,
            _state: PhantomData,
            clock_info: self.clock_info,
        })
    }
}

impl<PIN, E> DHT11<PIN, Initialized>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    /// Reads temperature and humidity from the sensor
    pub fn read_temp_and_hum(&mut self, delay: &mut impl DelayNs) -> error::Result<Measurement, E> {
        self.send_start_signal(delay)?;

        // Sensor response: 80µs Low followed by 80µs High
        self.wait_for_state(true, delay)?;  // End of Low phase
        self.wait_for_state(false, delay)?; // End of High phase

        // Skip the "Ghost Bit 👻" (the transition from handshake to data)
        self.wait_for_state(true, delay)?;
        self.wait_for_state(false, delay)?;

        // Decode durations into bits and bytes
        let mut data = [0u8; 5];
        for i in 0..40 {
            // 1. Messen der Low-Phase (Bit-Vorbereitung, ca. 50us)
            let low_duration = self.wait_for_state(true, delay)?;
            
            // 2. Messen der High-Phase (Die eigentliche Information)
            let high_duration = self.wait_for_state(false, delay)?;

            if self.is_bit_high(low_duration, high_duration) {
                data[i / 8] |= 1 << (7 - (i % 8));
            }
        }

        // Validate data using the 8-bit checksum
        let checksum_calculated = data[0]
            .wrapping_add(data[1])
            .wrapping_add(data[2])
            .wrapping_add(data[3]);

        if checksum_calculated != data[4] { return Err(DHT11Error::ChecksumMismatch); }

        // All good! Return the results
        Ok(Measurement {
            humidity: Humidity {
                integral: data[0],
                decimal: data[1],
            },
            temperature: Temperature {
                integral: data[2],
                decimal: data[3],
            },
            checksum: checksum_calculated, 
        })
    }
}