#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use core::future::Future;
use core::task::{Context, Poll, Waker};
use driver_core::{DriverKind, DriverManager, DriverState};
use embedded_hal::i2c::I2c as BlockingI2c;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::{Duration, Instant, Rate},
};
use esp_println::println;
use mpu6050_driver::{raw_to_g, Address, AsyncBus, Mpu6050Async};

const I2C_NAME: &str = "i2c0";
const SENSOR_NAME: &str = "mpu6050";
const I2C_FREQ_KHZ: u32 = 400;
const STARTUP_DELAY_MS: u64 = 200;
const READ_INTERVAL_MS: u64 = 100;

struct EspBlockingI2cBus<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> EspBlockingI2cBus<I2C> {
    const fn new(i2c: I2C, address: Address) -> Self {
        Self {
            i2c,
            address: address.as_u8(),
        }
    }
}

impl<I2C> AsyncBus for EspBlockingI2cBus<I2C>
where
    I2C: BlockingI2c,
{
    type Error = I2C::Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error> {
        let mut value = [0_u8; 1];

        self.i2c.write_read(self.address, &[reg], &mut value)?;

        Ok(value[0])
    }

    async fn write_reg(&mut self, reg: u8, value: u8) -> Result<(), Self::Error> {
        self.i2c.write(self.address, &[reg, value])
    }

    async fn read_multiple(&mut self, start_reg: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.i2c.write_read(self.address, &[start_reg], buffer)
    }
}

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(I2C_FREQ_KHZ));
    let i2c0 = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22);

    let mut manager: DriverManager<2> = DriverManager::new();

    if manager.register_primary(I2C_NAME, DriverKind::I2c).is_err() {
        fail("failed to register i2c0");
    }

    if manager.register_device(SENSOR_NAME, I2C_NAME).is_err() {
        fail("failed to register mpu6050");
    }

    let bus = EspBlockingI2cBus::new(i2c0, Address::Primary);
    let mut sensor = Mpu6050Async::new(bus);

    if block_on_ready(sensor.setup()).is_err() {
        let _ = manager.set_state(SENSOR_NAME, DriverState::Fault);
        fail("MPU-6050 setup failed");
    }

    if manager.set_state(SENSOR_NAME, DriverState::Ready).is_err() {
        fail("failed to set mpu6050 state");
    }

    let device_id = match block_on_ready(sensor.get_device_id()) {
        Ok(id) => id,
        Err(_) => fail("failed to read MPU-6050 device id"),
    };

    wait_ms(STARTUP_DELAY_MS);

    println!("MPU-6050 real-time acceleration");
    println!("WHO_AM_I: 0x{:02X}", device_id);
    println!("I2C: SDA=GPIO21, SCL=GPIO22, address=0x68");
    println!("Scale: +/-4g");
    println!("Move the GY-521 module and watch x/y/z change");
    println!("");

    loop {
        match block_on_ready(sensor.get_accel_raw()) {
            Ok(raw) => {
                let accel_g = raw_to_g(raw);

                println!(
                    "raw x={:>6} y={:>6} z={:>6} | g x={:>7.3} y={:>7.3} z={:>7.3}",
                    raw.0, raw.1, raw.2, accel_g.0, accel_g.1, accel_g.2
                );
            }
            Err(_) => {
                let _ = manager.set_state(SENSOR_NAME, DriverState::Fault);
                println!("failed to read MPU-6050 acceleration");
            }
        }

        wait_ms(READ_INTERVAL_MS);
    }
}

fn wait_ms(ms: u64) {
    let start = Instant::now();
    let duration = Duration::from_millis(ms);

    while start.elapsed() < duration {}
}

fn fail(message: &str) -> ! {
    println!("{message}");

    loop {
        wait_ms(1000);
    }
}

fn block_on_ready<F>(future: F) -> F::Output
where
    F: Future,
{
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = core::pin::pin!(future);

    match Future::poll(future.as_mut(), &mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => fail("future unexpectedly pending"),
    }
}
