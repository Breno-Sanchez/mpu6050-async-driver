# MPU-6050 Async Driver

Driver assíncrono, `no_std` e agnóstico de hardware para o MPU-6050 em Rust.

O projeto implementa um driver para acessar os subsistemas de acelerômetro, giroscópio e temperatura do MPU-6050 usando `embedded-hal-async`. A lógica do CI fica isolada no crate principal, enquanto a aplicação `app-esp32` serve apenas como exemplo real de integração com o ESP32 DevKit v1.

## Objetivo

O objetivo é manter o driver independente de microcontrolador, RTOS, HAL, GPIOs e executor assíncrono específico. O driver depende apenas de uma abstração de barramento, permitindo reutilização em outras plataformas que implementem as mesmas operações básicas de leitura e escrita.

## Estrutura

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

## Crate principal

O crate `mpu6050-async` contém somente a lógica genérica do dispositivo:

- mapa de registradores do MPU-6050;
- inicialização do CI;
- configuração do acelerômetro;
- configuração do giroscópio;
- configuração do filtro digital passa-baixa;
- leitura bruta de aceleração;
- leitura bruta de giroscópio;
- leitura bruta de temperatura;
- leitura conjunta de aceleração, temperatura e giroscópio;
- conversão de aceleração para `m/s²`;
- conversão do giroscópio para `dps`;
- conversão de temperatura para `°C`.

## Arquitetura agnóstica

O driver principal é parametrizado por um tipo genérico de barramento:

```rust
pub struct Mpu6050Async<XBUS> {
    bus: XBUS,
    accel_scale_factor: f32,
    gyro_scale_factor: f32,
}
```

Esse tipo precisa implementar a trait interna `AsyncBus`:

```rust
pub trait AsyncBus {
    type Error;

    async fn read_reg(&mut self, reg: u8) -> Result<u8, Self::Error>;
    async fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Self::Error>;
    async fn read_multiple(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Self::Error>;
    async fn write_multiple(&mut self, reg: u8, bytes: &[u8]) -> Result<(), Self::Error>;
}
```

Com isso, o driver não acessa diretamente periféricos, registradores da MCU, GPIOs, UART, clock tree, RTOS ou APIs específicas de fabricante. A camada de aplicação fornece apenas um adaptador entre o barramento real e essa interface abstrata.

## Barramento usado

O módulo GY-521 com MPU-6050 é usado via I2C. Nesta implementação não há suporte a SPI ou UART, pois o objetivo é refletir o barramento efetivamente usado pelo CI/módulo no hardware testado.

O crate principal fornece um adaptador `I2cBus<I2C>` para qualquer implementação compatível com `embedded-hal-async::i2c::I2c`.

## Aplicação ESP32

A pasta `app-esp32` contém somente o exemplo específico de placa. Ela configura o ESP32 DevKit v1 e cria um adaptador para usar o I2C real com o driver genérico.

Pinout utilizado:

| GY-521 / MPU-6050 | ESP32 DevKit v1 |
|---|---|
| VCC | 3V3 |
| GND | GND |
| SDA | GPIO21 |
| SCL | GPIO22 |
| AD0 | GND |

Com `AD0` em `GND`, o endereço I2C é `0x68`.

## Validar o driver

Na raiz do projeto:

```bash
cargo fmt
cargo check
```

## Compilar o exemplo ESP32

```bash
cd app-esp32
source "$HOME/export-esp.sh"
cargo +esp build --release --bin app-esp32
```

## Gravar e monitorar no ESP32

```bash
cd app-esp32
source "$HOME/export-esp.sh"
cargo +esp run --release --bin app-esp32
```

## Saída esperada

```text
MPU-6050 accelerometer and gyroscope
WHO_AM_I: 0x68
I2C: SDA=GPIO21, SCL=GPIO22, address=0x68
Accel: +/-4g | Gyro: +/-250 dps

accel m/s2 x=... y=... z=... | gyro dps x=... y=... z=... | temp C=...
```

## Observações

O driver é agnóstico em relação à plataforma, mas continua sendo específico para o MPU-6050. Para criar uma abstração comum para diferentes acelerômetros ou IMUs, seria necessário definir uma trait de nível superior, como `Accelerometer`, `Gyroscope` ou `Imu`, e implementar essa interface para cada CI.
