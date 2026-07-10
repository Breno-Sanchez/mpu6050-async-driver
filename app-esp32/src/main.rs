#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    i2c::master::{Config as I2cConfig, I2c},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println::println;
use mpu6050_async_driver::{Address, I2cBus, Mpu6050Async};

const I2C_FREQ_KHZ: u32 = 400;
const STARTUP_DELAY_MS: u64 = 200;
const READ_INTERVAL_MS: u64 = 100;
const HEARTBEAT_INTERVAL_MS: u64 = 1_000;

#[embassy_executor::task]
async fn heartbeat_task() {
    loop {
        Timer::after(Duration::from_millis(HEARTBEAT_INTERVAL_MS)).await;
    }
}

// ESP32 hardware example. The generic driver remains isolated in the root crate.
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    spawner.spawn(heartbeat_task().expect("failed to create heartbeat task"));

    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(I2C_FREQ_KHZ));

    let i2c0 = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22)
        .into_async();

    let bus = I2cBus::new(i2c0, Some(Address::PRIMARY));
    let mut sensor = Mpu6050Async::new(bus);

    if sensor.setup().await.is_err() {
        fail("MPU-6050 setup failed").await;
    }

    let device_id = match sensor.get_device_id().await {
        Ok(id) => id,
        Err(_) => fail("failed to read MPU-6050 device id").await,
    };

    let connected = match sensor.is_connected().await {
        Ok(value) => value,
        Err(_) => fail("failed to validate MPU-6050 connection").await,
    };

    if !connected {
        fail("invalid MPU-6050 WHO_AM_I").await;
    }

    Timer::after(Duration::from_millis(STARTUP_DELAY_MS)).await;

    println!("MPU-6050 full motion data initialized!");
    println!("WHO_AM_I: 0x{:02X}", device_id);
    println!("I2C: SDA=GPIO21, SCL=GPIO22, address=0x68");
    println!("Accel: +/-4g | Gyro: +/-250 dps");
    println!("Runtime: Embassy async executor + async I2C");
    println!("");

    loop {
        match sensor.get_motion().await {
            Ok(motion) => {
                println!(
                    "accel (m/s2): x={:>7.3} y={:>7.3} z={:>7.3} | gyro (dps): x={:>7.3} y={:>7.3} z={:>7.3} | temp: {:>6.2} C",
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

        Timer::after(Duration::from_millis(READ_INTERVAL_MS)).await;
    }
}

async fn fail(message: &str) -> ! {
    println!("{message}");

    loop {
        Timer::after(Duration::from_millis(1_000)).await;
    }
}
