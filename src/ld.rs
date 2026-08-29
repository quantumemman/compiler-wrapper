use std::env;
use std::string::String;
use std::process::{Command, ExitCode};
use wrapper::{Runtime, print_usage};

fn main() -> ExitCode {
    let _ = env_logger::try_init(); // Use try_init() to initialize the logger so the program doesn't crash if it's already initialized.
    if print_usage() {
        return ExitCode::SUCCESS;   // Print usage if WRAPPER_OPTIONS or WRAPPER_HELP env var is set and exit
    }
    let input_args: Vec<String> = env::args().skip(1).collect();  // grab all args except the first one (this program's name)
    let runtime = Runtime::new(file!().to_string(), input_args);  // create a wrapper Runtime struct to hold the runtime info
    runtime.print_info();                                         // print wrapper runtime info according to RUST_LOG

    // Run the target compiler/linker job and return the exit code
    let command_status = Command::new(&runtime.main_exe).args(&runtime.final_args).status().expect(&runtime.expect); // ld + processed args
    ExitCode::from(command_status.code().unwrap_or(1).clamp(0, 255) as u8)
}
