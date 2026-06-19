#![no_std]

use embedded_hal_async::i2c::I2c;

pub type AccelRaw = (i16, i16, i16);
pub type AccelG = (f32, f32, f32);

pub const EXPECTED_DEVICE_ID: u8 = 0x68;

const REG_SMPLRT_DIV: u8 = 0x19;
const REG_CONFIG: u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG: u8 = 0x1C;
const REG_ACCEL_XOUT_H: u8 = 0x3B;
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_WHO_AM_I: u8 = 0x75;

const PWR_MGMT_1_WAKE: u8 = 0x00;
const SMPLRT_DIV_125HZ: u8 = 0x07;
const CONFIG_DLPF_44HZ: u8 = 0x03;
const GYRO_CONFIG_250DPS: u8 = 0x00;
const ACCEL_CONFIG_4G: u8 = 0x08;
const ACCEL_SCALE_4G: f32 = 8192.0;

const CONFIG_SEQUENCE: &[(u8, u8)] = &[
    (REG_PWR_MGMT_1, PWR_MGMT_1_WAKE),
    (REG_SMPLRT_DIV, SMPLRT_DIV_125HZ),
    (REG_CONFIG, CONFIG_DLPF_44HZ),
    (REG_GYRO_CONFIG, GYRO_CONFIG_250DPS),
    (REG_ACCEL_CONFIG, ACCEL_CONFIG_4G),
];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Address {
    Primary = 0x68,
    Secondary = 0x69,
}

impl Address {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MpuError<E> {
    Bus(E),
    InvalidDeviceId(u8),
}

pub fn raw_to_g(raw: AccelRaw) -> AccelG {
    (
        raw.0 as f32 / ACCEL_SCALE_4G,
        raw.1 as f32 / ACCEL_SCALE_4G,
        raw.2 as f32 / ACCEL_SCALE_4G,
    )
}

#[allow(async_fn_in_trait)]
pub trait AsyncBus {
    type Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error>;

    async fn write_reg(&mut self, reg: u8, value: u8) -> Result<(), Self::Error>;

    async fn read_multiple(&mut self, start_reg: u8, buffer: &mut [u8]) -> Result<(), Self::Error>;
}

pub struct I2cBus<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> I2cBus<I2C> {
    pub const fn new(i2c: I2C, address: Address) -> Self {
        Self {
            i2c,
            address: address.as_u8(),
        }
    }
}

impl<I2C> AsyncBus for I2cBus<I2C>
where
    I2C: I2c,
{
    type Error = I2C::Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error> {
        let mut value = [0_u8; 1];

        self.i2c
            .write_read(self.address, &[reg], &mut value)
            .await?;

        Ok(value[0])
    }

    async fn write_reg(&mut self, reg: u8, value: u8) -> Result<(), Self::Error> {
        self.i2c.write(self.address, &[reg, value]).await
    }

    async fn read_multiple(&mut self, start_reg: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.i2c
            .write_read(self.address, &[start_reg], buffer)
            .await
    }
}

pub struct Mpu6050Async<BUS> {
    bus: BUS,
}

impl<BUS> Mpu6050Async<BUS>
where
    BUS: AsyncBus,
{
    pub const fn new(bus: BUS) -> Self {
        Self { bus }
    }

    pub async fn get_device_id(&mut self) -> Result<u8, BUS::Error> {
        self.bus.read_reg(REG_WHO_AM_I).await
    }

    pub async fn setup(&mut self) -> Result<(), MpuError<BUS::Error>> {
        let id = self.get_device_id().await.map_err(MpuError::Bus)?;

        if id != EXPECTED_DEVICE_ID {
            return Err(MpuError::InvalidDeviceId(id));
        }

        for &(reg, value) in CONFIG_SEQUENCE {
            self.bus
                .write_reg(reg, value)
                .await
                .map_err(MpuError::Bus)?;
        }

        Ok(())
    }

    pub async fn get_accel_raw(&mut self) -> Result<AccelRaw, BUS::Error> {
        let mut buffer = [0_u8; 6];

        self.bus
            .read_multiple(REG_ACCEL_XOUT_H, &mut buffer)
            .await?;

        Ok((
            i16::from_be_bytes([buffer[0], buffer[1]]),
            i16::from_be_bytes([buffer[2], buffer[3]]),
            i16::from_be_bytes([buffer[4], buffer[5]]),
        ))
    }
}
