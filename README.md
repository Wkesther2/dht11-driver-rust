# 🌡️ dht11-driver

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/Embedded-HAL-223311?style=for-the-badge&logo=cpu&logoColor=white" alt="embedded">
  <img src="https://img.shields.io/badge/STM32F4-Blue-blue?style=for-the-badge" alt="stm32">
</p>

---

## 📖 Table of Contents
- [📍 Overview](#-overview)
- [📦 Features](#-features)
- [📂 Repository Structure](#-repository-structure)
- [🚀 Getting Started](#-getting-started)
- [🛠 Technical Details](#-technical-details)
- [📃 License](#-license)

---

## 📍 Overview
This project provides a highly optimized, `no_std` Rust driver for the DHT11 temperature and humidity sensor.

Designed with safety and precision in mind, it supports both hardware-accelerated timing via ARM DWT and a portable software-based timing engine.

---

## 📦 Features

| Component | Description |
| :--- | :--- |
| **Agnostic** | **Platform-independent** core; works on any MCU via `embedded-hal`. |
| **Safety** | Uses **Typestates** to prevent reading from an uninitialized sensor. |
| **Precision** | **DWT hardware support** for sub-microsecond pulse measurement. |
| **Observability** | Native integration with `defmt` for zero-overhead logging. |

---

## 📂 Repository Structure

```sh
└── dht11-driver/
    ├── src/
    │   ├── lib.rs            # Core driver and Typestate logic
    │   └── examples/         # STM32F407 reference implementations
    │       ├── stm32f407_dwt.rs
    │       └── stm32f407_no_dwt.rs
    ├── .cargo/
    │   └── config.toml       # Runner & Linker settings
    └── Cargo.toml            # Feature gate definitions
```

## 🚀 Getting Started

### 1. Installation

Add the library to your `Cargo.toml`:

```toml
[dependencies]
dht11-driver = "0.1.2"
```

Or use the cargo CLI:

```
cargo add dht11-driver
```

### 2. Hardware Setup (STM32F407)

**1. Sensor Connection:** Connect the DHT11 DATA pin to PA0 on your STM32F407 Discovery/Black-Pill board.

**2. Pull-Up Resistor:** The DHT11 uses a single-wire bidirectional protocol. You must configure the pin as Open-Drain. A physical 4.7kΩ to 10kΩ pull-up resistor to VCC is highly recommended for signal integrity.

Running the Examples
This repository includes pre-configured examples using probe-rs. Use the following commands to flash your board:
| Mode | Command | Active Features |
| :--- | :--- | :--- |
| High Precision | cargo run --example stm32f407f_dwt --features use-dwt | use-dwt, use-defmt |
| Universal Timing | cargo run --example stm32407f_no_dwt | use-defmt |

## 🛠 Technical Details

**Typestate Safety**

To eliminate logic errors—such as attempting to read data before the sensor has finished its 2-second power-on stabilization—this driver implements the Typestate Pattern. The sensor object transitions through different types in your code:

```rust
// 1. Create the driver in 'Uninitialized' state
let dht11 = DHT11::new(pin, clocks.sysclk());

// 2. Initialize
// This consumes the old state and returns the 'Initialized' version
let mut dht11 = dht11.initialize(&mut delay).expect("Initialization failed");

// 3. Only now is the .read_temp_and_hum() method available in your IDE!
let measurement = dht11.read_temp_and_hum(&mut delay);
```

## 📖 License

Distributed under the **MIT License**. See the `LICENSE` file for more information.


