# MPU-6050 Async Driver

`mpu6050-async-driver` is a compact `no_std` asynchronous Rust driver for the MPU-6050 IMU. It exposes accelerometer, gyroscope and temperature readings through an `embedded-hal-async`-based bus abstraction.

The repository contains two parts:

- `mpu6050-async-driver`: the reusable, hardware-agnostic driver crate.
- `app-esp32`: an ESP32 DevKit v1 hardware example using Embassy/ESP async runtime and async I2C.

## Features

- `no_std` driver crate
- asynchronous register access
- hardware, HAL and RTOS independent driver core
- MPU-6050 accelerometer support
- MPU-6050 gyroscope support
- MPU-6050 temperature sensor support
- raw and converted readings
- combined 14-byte motion burst read
- ESP32 validation example with async I2C

## Repository layout

```text
.
├── app-esp32/
│   ├── build.rs
│   ├── Cargo.lock
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   └── src/main.rs
├── Cargo.lock
├── Cargo.toml
├── LICENSE
├── README.md
└── src/lib.rs
```

## Driver architecture

The driver is specific to the MPU-6050 register map, but it is not tied to a specific board, MCU, HAL or RTOS.

The core type is generic over the bus implementation:

```rust
pub struct Mpu6050Async<XBUS> {
    bus: XBUS,
    accel_scale_factor: f32,
    gyro_scale_factor: f32,
}
```

The bus only needs to implement the minimal asynchronous register interface required by the driver:

```rust
pub trait AsyncBus {
    type Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error>;
    async fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Self::Error>;
    async fn read_multiple(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Self::Error>;
    async fn write_multiple(&mut self, reg: u8, bytes: &[u8]) -> Result<(), Self::Error>;
}
```

For I2C devices, the crate provides `I2cBus<I2C>`, which wraps an `embedded-hal-async` I2C implementation.

## MPU-6050 configuration

The ESP32 example initializes the sensor with:

| Parameter | Value |
|---|---|
| I2C address | `0x68` |
| Accelerometer range | `±4g` |
| Gyroscope range | `±250 dps` |
| Sample rate divider | 125 Hz effective sample rate |
| DLPF | 44 Hz |
| Read interval | 100 ms |
| I2C frequency | 400 kHz |

The combined motion read starts at `ACCEL_XOUT_H` and reads 14 bytes in one burst:

```text
accel X/Y/Z -> temperature -> gyro X/Y/Z
```

Converted values are reported as:

| Measurement | Unit |
|---|---|
| Accelerometer | `m/s²` |
| Gyroscope | `dps` |
| Temperature | `°C` |

## ESP32 hardware example

The `app-esp32` crate validates the driver on an ESP32 DevKit v1. It uses:

- `#[esp_rtos::main]` async entry point
- Embassy executor
- `embassy_time::Timer`
- ESP async I2C via `.into_async()`
- `sensor.setup().await`
- `sensor.get_motion().await`

The example no longer uses blocking I2C, busy-wait delays or a manual `block_on` helper.

## Wiring

| GY-521 / MPU-6050 | ESP32 DevKit v1 |
|---|---|
| VCC | 3V3 |
| GND | GND |
| SDA | GPIO21 |
| SCL | GPIO22 |
| AD0 | GND |

With `AD0` connected to `GND`, the MPU-6050 uses I2C address `0x68`.

## Requirements

- Rust ESP toolchain configured
- ESP32 DevKit v1
- GY-521 / MPU-6050 module
- USB cable with serial support
- ESP environment export script available at `$HOME/export-esp.sh`

The tested target uses the `+esp` Rust toolchain.

## Check the generic driver

From the repository root:

```bash
cargo check
```

This checks the reusable `mpu6050-async-driver` crate.

## Build the ESP32 example

From the repository root:

```bash
cd app-esp32
source "$HOME/export-esp.sh"
cargo +esp build --release --bin app-esp32
```

This builds the ESP32 hardware example using the local driver crate.

## Flash and monitor

From `app-esp32`:

```bash
cargo +esp run --release --bin app-esp32
```

If the serial port is not detected automatically, set it explicitly:

```bash
ESPFLASH_PORT=/dev/ttyUSB0 cargo +esp run --release --bin app-esp32
```

or:

```bash
ESPFLASH_PORT=/dev/ttyACM0 cargo +esp run --release --bin app-esp32
```

## Expected output

```text
MPU-6050 full motion data initialized!
WHO_AM_I: 0x68
I2C: SDA=GPIO21, SCL=GPIO22, address=0x68
Accel: +/-4g | Gyro: +/-250 dps
Runtime: Embassy async executor + async I2C

accel (m/s2): x=  9.610 y=  0.645 z= -5.030 | gyro (dps): x= -2.481 y= -0.672 z=  1.221 | temp:  18.96 C
```

Values depend on sensor orientation, calibration and module quality.

## Notes

This is a driver for the MPU-6050. It is hardware-agnostic, not sensor-family-agnostic. The register map, scale factors and temperature formula are specific to the MPU-6050.

The ESP32 application is only a validation example. The reusable driver logic is isolated in the root crate.

## License

This project is licensed under the BSD Zero Clause License (`0BSD`). See [`LICENSE`](LICENSE).
