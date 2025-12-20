# DHT11 Driver in Rust 🌡️💧

A high-performance, `no_std` platform agnostic Rust driver for the DHT11 temperature and humidity sensor, specifically optimized for high-speed microcontrollers like the **STM32F407**.

---

## Features 🚀

* **High Precision:** Uses the **DWT (Data Watchpoint and Trace)** cycle counter for nanosecond-accurate pulse measurements at high clock speeds (168 MHz).
* **Safety First:** Implements the **Typestate Pattern** to ensure the sensor is properly initialized before any data is read. This prevents reading from the sensor during its unstable power-up phase. 🛡️
* **Memory Efficient:** Zero-allocation, `no_std` compatible, designed for bare-metal embedded systems.
* **Robust Error Handling:** Custom error types for Checksum mismatches, Timeouts, and Pin errors. 🕵️

---

## Project Status 🚧

| Feature | Status | Note |
| :--- | :--- | :--- |
| **DWT Mode** | ✅ Stable | Working perfectly at 168 MHz. |
| **Non-DWT Mode** | ⚠️ WIP | Software-based timing loop needs refactoring and better calibration. |
| **Refactoring** | 🔧 Planned | Modularizing internal timing and state logic for better maintainability. |
