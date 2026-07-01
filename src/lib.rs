#![no_std]

use embedded_hal_async::i2c::I2c;
use embedded_hal_async::i2c::Operation as I2cOperation;

// Registradores do MPU-6050
const REG_SMPLRT_DIV: u8 = 0x19;
const REG_CONFIG: u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG: u8 = 0x1C;
const REG_ACCEL_XOUT_H: u8 = 0x3B;
const REG_TEMP_OUT_H: u8 = 0x41;
const REG_GYRO_XOUT_H: u8 = 0x43;
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_WHO_AM_I: u8 = 0x75;

const EARTH_GRAVITY: f32 = 9.80665;
const EXPECTED_DEVICE_ID: u8 = 0x68;

#[derive(Copy, Clone, Debug)]
pub enum Address {
    PRIMARY = 0x68,
    SECONDARY = 0x69,
}

/// Formato dos dados do acelerômetro (G-Range)
#[derive(Copy, Clone, Debug)]
pub enum AccelRange {
    G2 = 0x00,
    G4 = 0x08,
    G8 = 0x10,
    G16 = 0x18,
}

/// Formato dos dados do giroscópio (DPS-Range)
#[derive(Copy, Clone, Debug)]
pub enum GyroRange {
    Dps250 = 0x00,
    Dps500 = 0x08,
    Dps1000 = 0x10,
    Dps2000 = 0x18,
}

/// Taxa de amostragem baseada no divisor interno do MPU-6050
#[derive(Copy, Clone, Debug)]
pub enum SampleRateDivider {
    Rate1kHz = 0x00,
    Rate500Hz = 0x01,
    Rate250Hz = 0x03,
    Rate125Hz = 0x07,
}

/// Configuração do filtro digital passa-baixa (DLPF)
#[derive(Copy, Clone, Debug)]
pub enum Dlpf {
    Hz260 = 0x00,
    Hz184 = 0x01,
    Hz94 = 0x02,
    Hz44 = 0x03,
    Hz21 = 0x04,
    Hz10 = 0x05,
    Hz5 = 0x06,
}

pub type AccelRaw = (i16, i16, i16);
pub type GyroRaw = (i16, i16, i16);
pub type Accel = (f32, f32, f32);
pub type Gyro = (f32, f32, f32);

#[derive(Copy, Clone, Debug)]
pub struct MotionRaw {
    pub accel: AccelRaw,
    pub temperature: i16,
    pub gyro: GyroRaw,
}

#[derive(Copy, Clone, Debug)]
pub struct Motion {
    pub accel: Accel,
    pub temperature_celsius: f32,
    pub gyro: Gyro,
}

// =========================================================================
// O DRIVER PRINCIPAL (Independente de protocolo)
// =========================================================================

pub struct Mpu6050Async<XBUS> {
    bus: XBUS,
    accel_scale_factor: f32,
    gyro_scale_factor: f32,
}

impl<XBUS> Mpu6050Async<XBUS>
where
    XBUS: AsyncBus,
{
    /// Cria uma nova instância do driver a partir de um Barramento (Bus) assíncrono
    pub fn new(bus: XBUS) -> Self {
        Self {
            bus,
            accel_scale_factor: 1.0 / 8192.0,
            gyro_scale_factor: 1.0 / 131.0,
        }
    }

    /// Lê o ID do dispositivo (Deve retornar 0x68)
    pub async fn get_device_id(&mut self) -> Result<u8, XBUS::Error> {
        self.bus.read_reg(REG_WHO_AM_I).await
    }

    /// Verifica se o dispositivo conectado é um MPU-6050
    pub async fn is_connected(&mut self) -> Result<bool, XBUS::Error> {
        let id = self.get_device_id().await?;
        Ok(id == EXPECTED_DEVICE_ID)
    }

    /// Inicializa o sensor, acorda o CI e configura acelerômetro e giroscópio
    pub async fn setup(&mut self) -> Result<(), XBUS::Error> {
        self.bus.write_reg(REG_PWR_MGMT_1, 0x00).await?;
        self.set_sample_rate(SampleRateDivider::Rate125Hz).await?;
        self.set_dlpf(Dlpf::Hz44).await?;
        self.set_accel_range(AccelRange::G4).await?;
        self.set_gyro_range(GyroRange::Dps250).await?;
        Ok(())
    }

    /// Configura a escala de leitura do acelerômetro
    pub async fn set_accel_range(&mut self, range: AccelRange) -> Result<(), XBUS::Error> {
        match range {
            AccelRange::G2 => self.accel_scale_factor = 1.0 / 16384.0,
            AccelRange::G4 => self.accel_scale_factor = 1.0 / 8192.0,
            AccelRange::G8 => self.accel_scale_factor = 1.0 / 4096.0,
            AccelRange::G16 => self.accel_scale_factor = 1.0 / 2048.0,
        }

        self.bus.write_reg(REG_ACCEL_CONFIG, range as u8).await?;
        Ok(())
    }

    /// Configura a escala de leitura do giroscópio
    pub async fn set_gyro_range(&mut self, range: GyroRange) -> Result<(), XBUS::Error> {
        match range {
            GyroRange::Dps250 => self.gyro_scale_factor = 1.0 / 131.0,
            GyroRange::Dps500 => self.gyro_scale_factor = 1.0 / 65.5,
            GyroRange::Dps1000 => self.gyro_scale_factor = 1.0 / 32.8,
            GyroRange::Dps2000 => self.gyro_scale_factor = 1.0 / 16.4,
        }

        self.bus.write_reg(REG_GYRO_CONFIG, range as u8).await?;
        Ok(())
    }

    /// Configura o filtro digital passa-baixa
    pub async fn set_dlpf(&mut self, dlpf: Dlpf) -> Result<(), XBUS::Error> {
        self.bus.write_reg(REG_CONFIG, dlpf as u8).await?;
        Ok(())
    }

    /// Configura o divisor da taxa de amostragem
    pub async fn set_sample_rate(&mut self, divider: SampleRateDivider) -> Result<(), XBUS::Error> {
        self.bus.write_reg(REG_SMPLRT_DIV, divider as u8).await?;
        Ok(())
    }

    /// Lê os três eixos brutos de aceleração
    pub async fn get_accel_raw(&mut self) -> Result<AccelRaw, XBUS::Error> {
        let mut buf = [0u8; 6];
        self.bus.read_multiple(REG_ACCEL_XOUT_H, &mut buf).await?;

        let x = i16::from_be_bytes([buf[0], buf[1]]);
        let y = i16::from_be_bytes([buf[2], buf[3]]);
        let z = i16::from_be_bytes([buf[4], buf[5]]);

        Ok((x, y, z))
    }

    /// Lê os três eixos brutos do giroscópio
    pub async fn get_gyro_raw(&mut self) -> Result<GyroRaw, XBUS::Error> {
        let mut buf = [0u8; 6];
        self.bus.read_multiple(REG_GYRO_XOUT_H, &mut buf).await?;

        let x = i16::from_be_bytes([buf[0], buf[1]]);
        let y = i16::from_be_bytes([buf[2], buf[3]]);
        let z = i16::from_be_bytes([buf[4], buf[5]]);

        Ok((x, y, z))
    }

    /// Lê a temperatura bruta
    pub async fn get_temperature_raw(&mut self) -> Result<i16, XBUS::Error> {
        let mut buf = [0u8; 2];
        self.bus.read_multiple(REG_TEMP_OUT_H, &mut buf).await?;
        Ok(i16::from_be_bytes([buf[0], buf[1]]))
    }

    /// Lê acelerômetro, temperatura e giroscópio em uma única rajada de 14 bytes
    pub async fn get_motion_raw(&mut self) -> Result<MotionRaw, XBUS::Error> {
        let mut buf = [0u8; 14];
        self.bus.read_multiple(REG_ACCEL_XOUT_H, &mut buf).await?;

        let accel = (
            i16::from_be_bytes([buf[0], buf[1]]),
            i16::from_be_bytes([buf[2], buf[3]]),
            i16::from_be_bytes([buf[4], buf[5]]),
        );

        let temperature = i16::from_be_bytes([buf[6], buf[7]]);

        let gyro = (
            i16::from_be_bytes([buf[8], buf[9]]),
            i16::from_be_bytes([buf[10], buf[11]]),
            i16::from_be_bytes([buf[12], buf[13]]),
        );

        Ok(MotionRaw {
            accel,
            temperature,
            gyro,
        })
    }

    /// Lê aceleração convertida para m/s²
    pub async fn get_accel(&mut self) -> Result<Accel, XBUS::Error> {
        let accel = self.get_accel_raw().await?;

        Ok((
            accel.0 as f32 * EARTH_GRAVITY * self.accel_scale_factor,
            accel.1 as f32 * EARTH_GRAVITY * self.accel_scale_factor,
            accel.2 as f32 * EARTH_GRAVITY * self.accel_scale_factor,
        ))
    }

    /// Lê giroscópio convertido para graus por segundo
    pub async fn get_gyro(&mut self) -> Result<Gyro, XBUS::Error> {
        let gyro = self.get_gyro_raw().await?;

        Ok((
            gyro.0 as f32 * self.gyro_scale_factor,
            gyro.1 as f32 * self.gyro_scale_factor,
            gyro.2 as f32 * self.gyro_scale_factor,
        ))
    }

    /// Lê temperatura convertida para Celsius
    pub async fn get_temperature(&mut self) -> Result<f32, XBUS::Error> {
        let temperature = self.get_temperature_raw().await?;
        Ok((temperature as f32 / 340.0) + 36.53)
    }

    /// Lê aceleração, temperatura e giroscópio convertidos
    pub async fn get_motion(&mut self) -> Result<Motion, XBUS::Error> {
        let motion = self.get_motion_raw().await?;

        let accel = (
            motion.accel.0 as f32 * EARTH_GRAVITY * self.accel_scale_factor,
            motion.accel.1 as f32 * EARTH_GRAVITY * self.accel_scale_factor,
            motion.accel.2 as f32 * EARTH_GRAVITY * self.accel_scale_factor,
        );

        let temperature_celsius = (motion.temperature as f32 / 340.0) + 36.53;

        let gyro = (
            motion.gyro.0 as f32 * self.gyro_scale_factor,
            motion.gyro.1 as f32 * self.gyro_scale_factor,
            motion.gyro.2 as f32 * self.gyro_scale_factor,
        );

        Ok(Motion {
            accel,
            temperature_celsius,
            gyro,
        })
    }
}

// =========================================================================
// CAMADA DE ABSTRAÇÃO DO BARRAMENTO
// =========================================================================

/// Trait interna que define as operações que qualquer barramento deve cumprir
#[allow(async_fn_in_trait)]
pub trait AsyncBus {
    type Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error>;
    async fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Self::Error>;
    async fn read_multiple(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Self::Error>;
    async fn write_multiple(&mut self, reg: u8, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// Implementação da abstração de barramento especificamente para I2C
pub struct I2cBus<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> I2cBus<I2C> {
    pub fn new(i2c: I2C, addr: Option<Address>) -> Self {
        Self {
            i2c,
            address: addr.map(|a| a as u8).unwrap_or(Address::PRIMARY as u8),
        }
    }
}

impl<I2C: I2c> AsyncBus for I2cBus<I2C> {
    type Error = I2C::Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(self.address, &[reg], &mut buf).await?;
        Ok(buf[0])
    }

    async fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Self::Error> {
        self.i2c.write(self.address, &[reg, val]).await?;
        Ok(())
    }

    async fn read_multiple(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Self::Error> {
        let cmd = [reg];

        let mut operations = [I2cOperation::Write(&cmd), I2cOperation::Read(buf)];

        self.i2c.transaction(self.address, &mut operations).await
    }

    async fn write_multiple(&mut self, reg: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let cmd = [reg];

        let mut operations = [I2cOperation::Write(&cmd), I2cOperation::Write(bytes)];

        self.i2c.transaction(self.address, &mut operations).await
    }
}
