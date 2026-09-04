use std::env;
use std::string::String;
use std::process::{Command, ExitCode};
use wrapper::{Runtime, check_help_flags, check_version_flags};

fn main() -> ExitCode {
    let _ = wrapper::init_logger();  // Initialize the dual logger (stdout + optional file)
    let src_executable = env::current_exe().unwrap().file_name().unwrap().to_str().unwrap().to_string();
    let input_args: Vec<String> = env::args().skip(1).collect();  // grab all args except the first one (this program's name)
    
    if !env::var("WRAPPER_ENABLE_PASSTHROUGH").is_ok() {
        if check_help_flags(&src_executable, &input_args) {
            return ExitCode::SUCCESS;   // Print usage if help flags are present and exit
        } else if check_version_flags(&src_executable, &input_args) {
            return ExitCode::SUCCESS;   // Print version if version flags are present and exit
        }
    }
    
    let runtime = Runtime::new(file!().to_string(), input_args);  // create a wrapper Runtime struct to hold the runtime info
    runtime.print_info();                                         // print wrapper runtime info according to RUST_LOG

    // Run the target compiler/linker job and return the exit code
    let command_status = Command::new(&runtime.main_exe).args(&runtime.final_args).status().expect(&runtime.expect); // g++ + processed args
    ExitCode::from(command_status.code().unwrap_or(1).clamp(0, 255) as u8)
}
