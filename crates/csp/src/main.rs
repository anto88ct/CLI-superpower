use std::process::ExitCode;

const USAGE: &str =
    "Uso: csp port <porta>\nMostra i processi in ascolto TCP sulla porta (1–65535).";

#[cfg(test)]
mod tests;

#[cfg(windows)]
mod port;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [help] if help == "--help" || help == "-h" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [command, help] if command == "port" && (help == "--help" || help == "-h") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [command, value] if command == "port" => {
            let Some(port) = value.to_str().and_then(parse_port) else {
                eprintln!("Porta non valida. Inserisci un intero tra 1 e 65535.");
                return ExitCode::from(2);
            };
            match run_port(port) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("csp port: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn parse_port(value: &str) -> Option<u16> {
    if value.bytes().all(|byte| byte.is_ascii_digit()) {
        value.parse().ok().filter(|port| *port > 0)
    } else {
        None
    }
}

#[cfg(windows)]
fn run_port(port: u16) -> Result<(), String> {
    port::run(port)
}

#[cfg(not(windows))]
fn run_port(_port: u16) -> Result<(), String> {
    Err("il comando è disponibile su Windows".into())
}
