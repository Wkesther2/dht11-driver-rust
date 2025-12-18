#![no_main]
#![no_std]

use embedded_hal::{delay::DelayNs, digital::{InputPin, OutputPin}};
use core::marker::PhantomData;

use defmt_rtt as _;

use panic_probe as _;

use crate::error::DHT11Error;

pub mod error {
    pub type Result<T, E> = core::result::Result<T, DHT11Error<E>>;

    #[derive(Debug, Clone, Copy, defmt::Format)]
    pub enum DHT11Error<E> {
        NoResponse,
        ChecksumMismatch,
        InvalidPulse,
        PinError(E),
        Timeout,
    }
}

pub struct Idle;
pub struct Ready;

pub struct DHT11<PIN, STATE> {
    pin: PIN,
    _state: PhantomData<STATE>,
}

impl<PIN, E, STATE> DHT11<PIN, STATE>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    fn wait_for_state(&mut self, target_low: bool, delay: &mut impl DelayNs, duration_us: u32) -> error::Result<u32, E> {
        let mut count = 0;
        while self.pin.is_low().map_err(DHT11Error::PinError)? != target_low {
            if count > duration_us {
                return Err(DHT11Error::Timeout);
            }
            delay.delay_us(1);
            count += 1;
        }

        Ok(count)
    }

    fn send_start_signal(&mut self, delay: &mut impl DelayNs) -> error::Result<(), E> {
        // Set Pin State for 18ms to LOW
        self.pin.set_low().map_err(DHT11Error::PinError)?;
        delay.delay_ms(18);
        
        // Set Pin State for 20-40ms to HIGH
        self.pin.set_high().map_err(DHT11Error::PinError)?;
        delay.delay_us(30);

        // Check if DHT11 pulls Pin State to LOW
        self.wait_for_state(true, delay, 100)?;

        Ok(())
    }
}

impl<PIN, E> DHT11<PIN, Idle>
where PIN: InputPin<Error = E> + OutputPin<Error = E>
{
    pub fn new(pin: PIN) -> Self {
        Self {
            pin,
            _state: PhantomData,
        }
    }

    pub fn initialize(mut self, delay: &mut impl DelayNs) -> error::Result<DHT11<PIN, Ready>, E> {
        // Wait for 1 Second to bypass the unsafe State of the DHT11
        delay.delay_ms(1000);

        // Test Handshake
        self.send_start_signal(delay)?;

        // Return updated Type
        Ok(
            DHT11 {
            pin: self.pin,
            _state: PhantomData,
        })
    }
}

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes a semihosting-capable debug tool exit
/// with status code 0.
pub fn exit() -> ! {
    semihosting::process::exit(0);
}

/// Hardfault handler.
///
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with an error. This seems better than the default, which is to spin in a
/// loop.
#[cortex_m_rt::exception]
unsafe fn HardFault(_frame: &cortex_m_rt::ExceptionFrame) -> ! {
    semihosting::process::exit(1);
}

// defmt-test 0.3.0 has the limitation that this `#[tests]` attribute can only be used
// once within a crate. the module can be in any file but there can only be at most
// one `#[tests]` module in this library crate
#[cfg(test)]
#[defmt_test::tests]
mod unit_tests {
    use defmt::assert;

    #[test]
    fn it_works() {
        assert!(true)
    }
}
