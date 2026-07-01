# MPU-6050 Async Driver

`mpu6050-async` is a `no_std` asynchronous Rust driver for the MPU-6050 inertial measurement unit. The driver exposes accelerometer, gyroscope and temperature readings through a hardware-agnostic bus abstraction based on `embedded-hal-async`.

The project follows the same architectural idea used in `adxl345-async`: the sensor logic lives in a reusable driver crate, while the board-specific code stays isolated in a separate hardware example.

## Project structure

```text
.
├── app-esp32/
│   ├── build.rs
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   └── src/main.rs
├── Cargo.toml
└── src/lib.rs
```

## Crates

### `mpu6050-async`

Root crate containing the generic MPU-6050 driver.

It includes:

- MPU-6050 register map;
- accelerometer configuration;
- gyroscope configuration;
- sample-rate configuration;
- DLPF configuration;
- raw accelerometer reads;
- raw gyroscope reads;
- temperature reads;
- combined motion reads;
- conversion to `m/s²`, `dps` and Celsius;
- generic asynchronous bus abstraction.

This crate is independent of ESP32, GPIOs, UART, RTOS, scheduler, bootloader and vendor-specific HAL APIs.

### `app-esp32`

Hardware validation example for ESP32 DevKit v1.

It configures the real I2C peripheral and adapts it to the generic bus interface required by the driver. This is the only board-specific part of the project.

## Architecture

The driver is generic over a bus type:

```rust
pub struct Mpu6050Async<XBUS> {
    bus: XBUS,
    accel_scale_factor: f32,
    gyro_scale_factor: f32,
}
```

The only requirement is that the bus implements the internal `AsyncBus` trait:

```rust
pub trait AsyncBus {
    type Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error>;
    async fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Self::Error>;
    async fn read_multiple(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Self::Error>;
    async fn write_multiple(&mut self, reg: u8, bytes: &[u8]) -> Result<(), Self::Error>;
}
```

This keeps the MPU-6050 logic independent from the concrete hardware implementation. To port the driver to another MCU or runtime, only a compatible bus adapter is required.

## MPU-6050 support

The driver supports the main internal measurement blocks of the MPU-6050:

- accelerometer;
- gyroscope;
- temperature sensor.

The combined motion read uses a single 14-byte burst starting at `ACCEL_XOUT_H`, reading accelerometer, temperature and gyroscope samples in sequence.

## Bus support

The current implementation targets I2C, which matches the GY-521 / MPU-6050 module used in the ESP32 validation setup.

UART is not implemented because it is not a native communication interface for this sensor. SPI is also not included in this project because the tested MPU-6050/GY-521 setup uses I2C.

## ESP32 wiring

| GY-521 / MPU-6050 | ESP32 DevKit v1 |
|---|---|
| VCC | 3V3 |
| GND | GND |
| SDA | GPIO21 |
| SCL | GPIO22 |
| AD0 | GND |

With `AD0` connected to `GND`, the I2C address is `0x68`.

## Build the generic driver

Run from the repository root:

```bash
cargo check
```

This checks only the generic `mpu6050-async` crate.

## Build the ESP32 example

Run from the repository root:

```bash
source "$HOME/export-esp.sh"
cd app-esp32
cargo +esp build --release --bin app-esp32
```

This builds the ESP32 hardware example using the generic driver from the root crate.

## Flash and monitor the ESP32

Run inside `app-esp32`:

```bash
cargo +esp run --release --bin app-esp32
```

Expected output:

```text
MPU-6050 accelerometer and gyroscope
WHO_AM_I: 0x68
I2C: SDA=GPIO21, SCL=GPIO22, address=0x68
Accel: +/-4g | Gyro: +/-250 dps

accel m/s2 x=... y=... z=... | gyro dps x=... y=... z=... | temp C=...
```

## Design summary

The project removes the previous external driver manager and keeps the driver model idiomatic to Rust. The Rust type system and trait bounds express the dependency between the MPU-6050 driver and the bus abstraction without requiring a manual driver registry.

The result is a smaller, clearer and more portable embedded driver:

- `no_std`;
- async;
- MCU-agnostic;
- RTOS-agnostic;
- HAL-agnostic;
- I2C-based for the tested MPU-6050 module;
- accelerometer and gyroscope support in the same driver.
