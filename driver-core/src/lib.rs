#![no_std]

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DriverKind {
    I2c,
    Spi,
    Uart,
    Device,
}

impl DriverKind {
    pub const fn is_primary(self) -> bool {
        match self {
            Self::I2c | Self::Spi | Self::Uart => true,
            Self::Device => false,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DriverState {
    Uninitialized,
    Ready,
    Fault,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DriverInfo {
    pub name: &'static str,
    pub kind: DriverKind,
    pub state: DriverState,
    pub dependency: Option<&'static str>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DriverError {
    Full,
    Duplicate,
    NotFound,
    MissingDependency,
    InvalidDependency,
}

pub struct DriverManager<const N: usize> {
    drivers: [Option<DriverInfo>; N],
    len: usize,
}

impl<const N: usize> DriverManager<N> {
    pub const fn new() -> Self {
        Self {
            drivers: [None; N],
            len: 0,
        }
    }

    pub fn register_primary(
        &mut self,
        name: &'static str,
        kind: DriverKind,
    ) -> Result<(), DriverError> {
        if !kind.is_primary() {
            return Err(DriverError::InvalidDependency);
        }

        self.insert(DriverInfo {
            name,
            kind,
            state: DriverState::Ready,
            dependency: None,
        })
    }

    pub fn register_device(
        &mut self,
        name: &'static str,
        dependency: &'static str,
    ) -> Result<(), DriverError> {
        let dependency_info = self.get(dependency).ok_or(DriverError::MissingDependency)?;

        if !dependency_info.kind.is_primary() {
            return Err(DriverError::InvalidDependency);
        }

        self.insert(DriverInfo {
            name,
            kind: DriverKind::Device,
            state: DriverState::Uninitialized,
            dependency: Some(dependency),
        })
    }

    pub fn register(&mut self, driver: DriverInfo) -> Result<(), DriverError> {
        match driver.kind {
            DriverKind::Device => {
                let dependency = driver.dependency.ok_or(DriverError::MissingDependency)?;
                let dependency_info = self.get(dependency).ok_or(DriverError::MissingDependency)?;

                if !dependency_info.kind.is_primary() {
                    return Err(DriverError::InvalidDependency);
                }
            }
            DriverKind::I2c | DriverKind::Spi | DriverKind::Uart => {
                if driver.dependency.is_some() {
                    return Err(DriverError::InvalidDependency);
                }
            }
        }

        self.insert(driver)
    }

    pub fn set_state(&mut self, name: &str, state: DriverState) -> Result<(), DriverError> {
        let driver = self
            .drivers
            .iter_mut()
            .flatten()
            .find(|driver| driver.name == name)
            .ok_or(DriverError::NotFound)?;

        driver.state = state;

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&DriverInfo> {
        self.drivers
            .iter()
            .flatten()
            .find(|driver| driver.name == name)
    }

    pub fn list(&self) -> impl Iterator<Item = &DriverInfo> {
        self.drivers.iter().flatten()
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn insert(&mut self, driver: DriverInfo) -> Result<(), DriverError> {
        if self.get(driver.name).is_some() {
            return Err(DriverError::Duplicate);
        }

        let slot = self
            .drivers
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(DriverError::Full)?;

        *slot = Some(driver);
        self.len += 1;

        Ok(())
    }
}

impl<const N: usize> Default for DriverManager<N> {
    fn default() -> Self {
        Self::new()
    }
}
