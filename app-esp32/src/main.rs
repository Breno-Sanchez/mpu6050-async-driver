#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use core::future::Future;
use core::task::{Context, Poll, Waker};
use embedded_hal::i2c::I2c as BlockingI2c;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::{Duration, Instant, Rate},
};
use esp_println::println;
use mpu6050_async::{Address, AsyncBus, Mpu6050Async};

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
            address: address as u8,
        }
    }
}

impl<I2C> AsyncBus for EspBlockingI2cBus<I2C>
where
    I2C: BlockingI2c,
{
    type Error = I2C::Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(self.address, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    async fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Self::Error> {
        self.i2c.write(self.address, &[reg, val])
    }

    async fn read_multiple(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.i2c.write_read(self.address, &[reg], buf)
    }

    async fn write_multiple(&mut self, reg: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut data = [0u8; 16];
        data[0] = reg;

        let len = bytes.len();

        for index in 0..len {
            data[index + 1] = bytes[index];
        }

        self.i2c.write(self.address, &data[..len + 1])
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

    let bus = EspBlockingI2cBus::new(i2c0, Address::PRIMARY);
    let mut sensor = Mpu6050Async::new(bus);

    if block_on_ready(sensor.setup()).is_err() {
        fail("MPU-6050 setup failed");
    }

    let device_id = match block_on_ready(sensor.get_device_id()) {
        Ok(id) => id,
        Err(_) => fail("failed to read MPU-6050 device id"),
    };

    let connected = match block_on_ready(sensor.is_connected()) {
        Ok(value) => value,
        Err(_) => fail("failed to validate MPU-6050 device id"),
    };

    if !connected {
        fail("invalid MPU-6050 WHO_AM_I");
    }

    wait_ms(STARTUP_DELAY_MS);

    println!("MPU-6050 accelerometer and gyroscope");
    println!("WHO_AM_I: 0x{:02X}", device_id);
    println!("I2C: SDA=GPIO21, SCL=GPIO22, address=0x68");
    println!("Accel: +/-4g | Gyro: +/-250 dps");
    println!("");

    loop {
        match block_on_ready(sensor.get_motion()) {
            Ok(motion) => {
                println!(
                    "accel m/s2 x={:>7.3} y={:>7.3} z={:>7.3} | gyro dps x={:>7.3} y={:>7.3} z={:>7.3} | temp C={:>6.2}",
                    motion.accel.0,
                    motion.accel.1,
                    motion.accel.2,
                    motion.gyro.0,
                    motion.gyro.1,
                    motion.gyro.2,
                    motion.temperature_celsius,
                );
            }
            Err(_) => {
                println!("failed to read MPU-6050 motion data");
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
