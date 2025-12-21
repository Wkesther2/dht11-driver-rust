#![deny(unsafe_code)]
#![cfg_attr(not(test), no_std)]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin}
};
use core::marker::PhantomData;

#[cfg(feature = "dwt")]
use cortex_m::peripheral::DWT;

use crate::error::DHT11Error;

/// ⏱️ Max time to wait for a state change before giving up
const TIMEOUT_US: u16 = 200;

pub mod error {
    pub type Result<T, E> = core::result::Result<T, DHT11Error<E>>;

    #[derive(Debug, Clone, Copy, defmt::Format)]
    pub enum DHT11Error<E> {
        NoResponse,       // 🤐 Sensor didn't answer the start signal
        ChecksumMismatch, // 🧮 Data was received but is corrupted
        PinError(E),      // 🔌 Hardware/GPIO issue
        Timeout,          // ⏳ Sensor took too long to change state
    }
}

// --- Typestate Pattern for Safety 🛡️ ---
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
}

impl<PIN, E, STATE> DHT11<PIN, STATE>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    /// 🕵️ Waits for the pin to reach a certain state and measures the time taken
    fn wait_for_state(&mut self, target_high: bool, _delay: &mut impl DelayNs) -> error::Result<u32, E> {
        #[cfg(feature = "dwt")]
        {
            let start = DWT::cycle_count();
            // Loop until the pin state matches our target
            while self.pin.is_low().map_err(DHT11Error::PinError)? == target_high {
                if DWT::cycle_count().wrapping_sub(start) > (TIMEOUT_US * 168) as u32 {
                    return Err(DHT11Error::Timeout);
                }
            }
            Ok(DWT::cycle_count().wrapping_sub(start))
        }

        #[cfg(not(feature = "dwt"))]
        {
            let mut count = 0;
            while self.pin.is_low().map_err(DHT11Error::PinError)? == target_high {
                count += 1;
                if count > TIMEOUT_US { return Err(DHT11Error::Timeout); }
                _delay.delay_us(1);
            }
            Ok(count as u32)
        }
    }

    /// 📣 Sends the 18ms pulse to wake up the sensor
    fn send_start_signal(&mut self, delay: &mut impl DelayNs) -> error::Result<(), E> {
        self.pin.set_high().map_err(DHT11Error::PinError)?;
        delay.delay_ms(1);

        // Pull low for 18-20ms to signal the start 📉
        self.pin.set_low().map_err(DHT11Error::PinError)?;
        delay.delay_ms(20);
        
        // Release the line and wait for the sensor to pull it low 📈
        self.pin.set_high().map_err(DHT11Error::PinError)?;

        // Wait for the sensor's acknowledgment (LOW)
        self.wait_for_state(false, delay)?;

        Ok(())
    }
}

impl<PIN, E> DHT11<PIN, Idle>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    pub fn new(pin: PIN) -> DHT11<PIN, Uninitialized> {
        DHT11 {
            pin,
            _state: PhantomData,
        }
    }

    /// Creates and initializes the sensor in one go.
    /// ⚠️ WARNING: This is a blocking call that takes ~3 seconds to complete.
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
    /// 🛠️ Initializes the sensor with required power-on delays
    /// ⚠️ WARNING: Wait at least 2 seconds after initialization before reading the sensor
    pub fn initialize(mut self, delay: &mut impl DelayNs) -> error::Result<DHT11<PIN, Initialized>, E> {
        // Perform a test handshake to ensure the sensor is present 🤝
        self.send_start_signal(delay)?;

        Ok(DHT11 {
            pin: self.pin,
            _state: PhantomData,
        })
    }
}

impl<PIN, E> DHT11<PIN, Initialized>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    /// 🌡️ Reads temperature and humidity from the sensor
    pub fn read_temp_and_hum(&mut self, delay: &mut impl DelayNs) -> error::Result<Measurement, E> {
        self.send_start_signal(delay)?;

        // Sensor response: 80µs Low followed by 80µs High
        self.wait_for_state(true, delay)?;  // End of Low phase
        self.wait_for_state(false, delay)?; // End of High phase

        // 👻 Skip the "Ghost Bit" (the transition from handshake to data)
        self.wait_for_state(true, delay)?;
        self.wait_for_state(false, delay)?;

        // Capture raw high-pulse durations for 40 bits
        // We do this as fast as possible to avoid timing jitter
        let mut high_durations = [0u32; 40];
        for val in high_durations.iter_mut() {
            self.wait_for_state(true, delay)?; // Wait for the high flank
            *val = self.wait_for_state(false, delay)?; // Measure the high pulse duration
        }

        // Decode durations into bits and bytes
        let mut data = [0u8; 5];
        for (i, val) in high_durations.iter().enumerate() {
            data[i / 8] <<= 1;
            
            #[cfg(feature = "dwt")]
            let is_one = *val > 8000; // Threshold: Logic 0 (~4000) vs Logic 1 (~11700)
            
            #[cfg(not(feature = "dwt"))]
            let is_one = *val > 40;

            if is_one {
                data[i / 8] |= 1;
            }
        }

        // 🧮 Validate data using the 8-bit checksum
        let checksum_calculated = data[0]
            .wrapping_add(data[1])
            .wrapping_add(data[2])
            .wrapping_add(data[3]);

        if checksum_calculated != data[4] {
            defmt::info!("Checksum Error! Expected: {}, Calculated: {}", data[4], checksum_calculated);
            defmt::debug!("Raw High Durations: {:?}", high_durations);
            return Err(DHT11Error::ChecksumMismatch);
        }

        // 🎉 All good! Return the results
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