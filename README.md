# Sistema de Driver Genérico Agnóstico de RTOS

Projeto em Rust para demonstrar uma arquitetura de drivers embarcados com:

- gerenciamento de drivers primários, como I2C, SPI e UART;
- gerenciamento de drivers de dispositivo dependentes de drivers primários;
- driver de dispositivo MPU-6050 independente de RTOS e de microcontrolador;
- aplicação real em ESP32 DevKit v1 lendo o módulo GY-521/MPU-6050 via I2C.

O objetivo principal é separar o código genérico do driver da camada específica de hardware. Assim, o núcleo do projeto pode ser reutilizado em outros microcontroladores ou ambientes de execução, desde que seja fornecida uma adaptação compatível para o barramento usado.

---

## Requisitos atendidos

| Requisito | Implementação |
|---|---|
| Driver agnóstico de RTOS | O `mpu6050-driver` não depende de FreeRTOS, Embassy, RTIC, ESP-IDF ou qualquer scheduler específico. |
| Gerenciamento de drivers primários | O `driver-core` representa drivers primários por meio de `DriverKind::I2c`, `DriverKind::Spi` e `DriverKind::Uart`. |
| Gerenciamento de dispositivos dependentes | O `driver-core` permite registrar um dispositivo, como `mpu6050`, associado a um driver primário, como `i2c0`. |
| Separação entre driver e aplicação | O driver do MPU-6050 não conhece ESP32, GPIO, UART, monitor serial ou detalhes da placa. |
| Demonstração real em hardware | O `app-esp32` inicializa o ESP32, configura I2C e lê o GY-521/MPU-6050 em tempo real. |

---

## Estrutura do projeto

```text
Driver/
├── app-esp32/
│   ├── build.rs
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   └── src/main.rs
├── driver-core/
│   ├── Cargo.toml
│   ├── src/lib.rs
│   └── tests/manager.rs
├── mpu6050-driver/
│   ├── Cargo.toml
│   └── src/lib.rs
├── xtask/
│   ├── Cargo.toml
│   └── src/main.rs
├── Cargo.toml
├── Cargo.lock
├── README.md
└── .gitignore
```

### `driver-core/`

Contém o gerenciador genérico de drivers. Ele registra drivers primários e dispositivos dependentes, além de armazenar metadados como nome, tipo, estado e dependência.

Exemplo conceitual:

```rust
manager.register_primary("i2c0", DriverKind::I2c);
manager.register_device("mpu6050", "i2c0");
```

Nesse exemplo, `i2c0` é um driver primário e `mpu6050` é um driver de dispositivo dependente do barramento I2C.

### `mpu6050-driver/`

Contém o driver genérico do MPU-6050. Esse crate é `no_std` e usa uma abstração de barramento compatível com `embedded-hal-async`.

O driver implementa:

- leitura do registrador `WHO_AM_I`;
- configuração inicial do sensor;
- leitura dos eixos do acelerômetro;
- conversão dos valores brutos para unidade `g`;
- escala de aceleração configurada para `±4g`.

### `app-esp32/`

Contém a aplicação específica para ESP32 DevKit v1. Essa camada inicializa o hardware real, configura o I2C e adapta o barramento do ESP32 para a trait usada pelo driver genérico.

Essa é a única parte do projeto que depende diretamente de:

- `esp-hal`;
- `esp-backtrace`;
- `esp-println`;
- GPIOs específicos do ESP32;
- bootloader/descritor de aplicação ESP-IDF.

### `xtask/`

Contém comandos auxiliares para padronizar a compilação, validação, limpeza e gravação do firmware.

---

## Arquitetura

A arquitetura é organizada em três camadas:

```text
Aplicação específica da placa
        ↓
Adaptação do barramento real
        ↓
Driver genérico do dispositivo
        ↓
Gerenciador genérico de drivers
```

No caso deste projeto:

```text
app-esp32
        ↓
EspBlockingI2cBus
        ↓
mpu6050-driver
        ↓
driver-core
```

O ponto central é que o `mpu6050-driver` não acessa diretamente nenhum periférico do ESP32. Ele só conhece a trait `AsyncBus`, que define operações genéricas de leitura e escrita em registradores.

Isso permite reaproveitar o driver em outra plataforma, desde que seja criada uma adaptação equivalente para o barramento dessa plataforma.

---

## Ligação elétrica

| GY-521 / MPU-6050 | ESP32 DevKit v1 |
|---|---|
| VCC | 3V3 |
| GND | GND |
| SDA | GPIO21 |
| SCL | GPIO22 |
| AD0 | GND |

Com `AD0` ligado ao `GND`, o endereço I2C usado é `0x68`.

> Observação: recomenda-se alimentar o módulo em `3V3` para manter o nível lógico do I2C compatível com o ESP32.

---

## Saída esperada

Ao executar o firmware, o monitor serial deve exibir uma saída semelhante a:

```text
MPU-6050 real-time acceleration
WHO_AM_I: 0x68
I2C: SDA=GPIO21, SCL=GPIO22, address=0x68
Scale: +/-4g
Move the GY-521 module and watch x/y/z change

raw x= -7293 y=-10369 z=   340 | g x= -0.890 y= -1.266 z=  0.042
raw x=  5172 y= -9960 z=  7874 | g x=  0.631 y= -1.216 z=  0.961
raw x= -32768 y= 32767 z=-32768 | g x= -4.000 y=  4.000 z= -4.000
```

Os valores `raw` são as leituras brutas do acelerômetro. Os valores em `g` são calculados usando a escala `±4g`, em que `8192 LSB` correspondem a aproximadamente `1g`.

Valores próximos de `±4.000g` indicam saturação da escala, o que pode ocorrer quando o módulo é movimentado com força.

---

## Comandos principais

Todos os comandos abaixo devem ser executados na raiz do projeto.

### Validar o projeto

```bash
cargo ci
```

Esse comando formata o código, verifica o workspace, executa os testes do `driver-core`, roda o Clippy e compila o firmware ESP32 em modo `release`.

### Compilar apenas o firmware ESP32

```bash
cargo esp-build
```

Esse comando compila o firmware do ESP32 sem gravar na placa.

### Gravar e monitorar o ESP32

```bash
cargo esp-run
```

Esse comando compila, grava o firmware no ESP32 e abre o monitor serial.

### Abrir somente o monitor serial

```bash
cargo esp-monitor
```

Esse comando abre o monitor serial sem recompilar nem gravar novamente.

### Limpar arquivos de compilação

```bash
cargo clean-all
```

Esse comando remove os diretórios de compilação `target/` e `app-esp32/target/`.

---

## Arquivos de compilação

As pastas abaixo são geradas automaticamente pelo Cargo e não fazem parte do código-fonte:

```text
target/
app-esp32/target/
```

Elas podem ser removidas antes de enviar o projeto, pois serão recriadas em uma nova compilação.

Os arquivos `Cargo.lock` foram mantidos para facilitar a reprodução das versões exatas das dependências usadas no projeto.

---

## Como gerar um pacote limpo para envio

A partir da pasta acima do projeto:

```bash
zip -r Driver_final.zip Driver \
  -x '*/target/*' \
  -x '*/.git/*' \
  -x '*.zip'
```

Esse comando cria um arquivo `.zip` contendo o código-fonte e os arquivos de configuração, sem incluir artefatos de compilação.

---

## Conclusão

O projeto demonstra um sistema de driver genérico e agnóstico de RTOS, com separação clara entre:

- gerenciamento de drivers;
- driver de dispositivo;
- adaptação de barramento;
- aplicação específica da placa.

A implementação foi validada em hardware real com ESP32 DevKit v1 e módulo GY-521/MPU-6050, realizando leitura em tempo real dos eixos do acelerômetro via I2C.
