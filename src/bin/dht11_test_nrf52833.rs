#![no_main]
#![no_std]

use dht11_driver_rust as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world!");

    dht11_driver_rust::exit()
}
