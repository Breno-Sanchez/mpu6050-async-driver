use driver_core::{DriverError, DriverInfo, DriverKind, DriverManager, DriverState};

#[test]
fn registers_primary_and_dependent_driver() {
    let mut manager: DriverManager<2> = DriverManager::new();

    manager.register_primary("i2c0", DriverKind::I2c).unwrap();
    manager.register_device("mpu6050", "i2c0").unwrap();

    assert_eq!(manager.len(), 2);
    assert_eq!(manager.get("i2c0").unwrap().dependency, None);
    assert_eq!(manager.get("mpu6050").unwrap().dependency, Some("i2c0"));
}

#[test]
fn rejects_device_with_missing_dependency() {
    let mut manager: DriverManager<2> = DriverManager::new();

    let result = manager.register_device("mpu6050", "i2c0");

    assert_eq!(result, Err(DriverError::MissingDependency));
}

#[test]
fn rejects_duplicate_driver_name() {
    let mut manager: DriverManager<2> = DriverManager::new();

    manager.register_primary("i2c0", DriverKind::I2c).unwrap();

    let result = manager.register_primary("i2c0", DriverKind::I2c);

    assert_eq!(result, Err(DriverError::Duplicate));
}

#[test]
fn rejects_when_manager_is_full() {
    let mut manager: DriverManager<1> = DriverManager::new();

    manager.register_primary("i2c0", DriverKind::I2c).unwrap();

    let result = manager.register_primary("spi0", DriverKind::Spi);

    assert_eq!(result, Err(DriverError::Full));
}

#[test]
fn updates_driver_state() {
    let mut manager: DriverManager<2> = DriverManager::new();

    manager.register_primary("i2c0", DriverKind::I2c).unwrap();
    manager.register_device("mpu6050", "i2c0").unwrap();

    manager.set_state("mpu6050", DriverState::Ready).unwrap();

    assert_eq!(manager.get("mpu6050").unwrap().state, DriverState::Ready);
}

#[test]
fn validates_register_with_full_driver_info() {
    let mut manager: DriverManager<2> = DriverManager::new();

    manager
        .register(DriverInfo {
            name: "i2c0",
            kind: DriverKind::I2c,
            state: DriverState::Ready,
            dependency: None,
        })
        .unwrap();

    manager
        .register(DriverInfo {
            name: "mpu6050",
            kind: DriverKind::Device,
            state: DriverState::Uninitialized,
            dependency: Some("i2c0"),
        })
        .unwrap();

    assert_eq!(manager.len(), 2);
}
