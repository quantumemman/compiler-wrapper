use std::env;
use std::string::String;
use std::process::{Command, ExitCode};
use wrapper::{Runtime, help_message, version_message};

/// Single shared entry point for every wrapper binary.
/// Identity comes from Cargo.toml `[[bin]] name` (env! expands at compile time
/// per binary). Keeps the `-rs` suffix so PROJECT_SIGNATURE self-detection keeps
/// working; `pp` -> `++` normalization happens inside Runtime (see BIN_NAME_ALIASES).
fn main() -> ExitCode {
    let _ = wrapper::init_logger();                               // Initialize the dual logger (stdout + optional file)
    let input_args: Vec<String> = env::args().skip(1).collect();  // grab all args except the first one (this program's name)
    let runtime = Runtime::new(env!("CARGO_BIN_NAME").to_string(), input_args);  // create a wrapper Runtime struct to hold the runtime info

    // Log various items based on the options specified in the environment variables
    if version_message(&runtime) { return ExitCode::SUCCESS; }    // Print version if version flags are present and exit
    else if help_message(&runtime) { return ExitCode::SUCCESS; }  // Print usage if help flags are present and exit
    runtime.print_info();                                         // print wrapper runtime info according to RUST_LOG

    // Run the target compiler/linker job and return the exit code.
    // Set WRAPPER_ENABLE_PASSTHROUGH=1 for the child process to avoid duplicate flag processing.
    let command_status = Command::new(&runtime.main_exe).args(&runtime.final_args).env("WRAPPER_ENABLE_PASSTHROUGH","1").status().expect(&runtime.expect);
    ExitCode::from(command_status.code().unwrap_or(1).clamp(0, 255) as u8)
}
