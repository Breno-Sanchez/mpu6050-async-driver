use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let command = env::args().nth(1).unwrap_or_else(|| "help".to_string());

    let result = match command.as_str() {
        "help" => help(),
        "fmt" => run("cargo", &["fmt", "--all"]),
        "check" => check(),
        "ci" => ci(),
        "clean" => clean(),
        "size" => size(),
        "esp-build" => esp("cargo +esp build --release --bin app-esp32"),
        "esp-run" => esp("cargo +esp run --release --bin app-esp32"),
        "esp-monitor" => esp("espflash monitor"),
        _ => Err(format!("Comando desconhecido: {command}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn help() -> Result<(), String> {
    println!("Comandos disponíveis:");
    println!("  cargo check-all   -> fmt check + check + clippy");
    println!("  cargo ci          -> fmt + check + clippy + build ESP32");
    println!("  cargo clean-all   -> limpa target/");
    println!("  cargo size-src    -> mostra tamanho do código");
    println!("  cargo esp-build   -> compila o firmware");
    println!("  cargo esp-run     -> grava e monitora o ESP32");
    println!("  cargo esp-monitor -> abre o monitor serial");
    Ok(())
}

fn check() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run("cargo", &["check", "--workspace"])?;
    run("cargo", &["test", "-p", "driver-core"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )
}

fn ci() -> Result<(), String> {
    run("cargo", &["fmt", "--all"])?;
    run("cargo", &["check", "--workspace"])?;
    run("cargo", &["test", "-p", "driver-core"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    esp("cargo +esp build --release --bin app-esp32")
}

fn clean() -> Result<(), String> {
    run("cargo", &["clean"])?;
    remove_dir("app-esp32/target")
}

fn size() -> Result<(), String> {
    run(
        "du",
        &["-sh", "--exclude=target", "--exclude=app-esp32/target", "."],
    )
}

fn esp(command: &str) -> Result<(), String> {
    if !Path::new("app-esp32").exists() {
        return Err("app-esp32 não existe.".to_string());
    }

    let shell_command = format!("source $HOME/export-esp.sh && cd app-esp32 && {command}");

    run("bash", &["-lc", &shell_command])
}

fn remove_dir(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        fs::remove_dir_all(path).map_err(|error| format!("Falha ao remover {path}: {error}"))?;
    }

    Ok(())
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    println!("$ {} {}", program, args.join(" "));

    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("Falha ao executar {program}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Comando falhou: {program} {}", args.join(" ")))
    }
}
