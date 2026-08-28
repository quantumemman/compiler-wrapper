use std::env;
use std::string::String;
use std::process::{Command, ExitCode};
use wrapper::{Kind};

fn main() -> ExitCode {
    let _ = env_logger::try_init(); // Use try_init() to initialize the logger so the program doesn't crash if it's already initialized.
    let args: Vec<String> = env::args().skip(1).collect();                                          // all of 'em
    let kind = Kind::new(file!().to_string(), args);
    let status = Command::new(&kind.getEXE()).args(&kind.finalArgs).status().expect(&kind.expect); // gcc + cleaned args

    kind.printInfo();
    ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8)
}
