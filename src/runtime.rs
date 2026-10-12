use log::{trace, debug, info, warn};
use crate::constants::{NONE_KEYWORD};
use crate::classify::{ExecutableFamily, ExecutableKind, get_target_classification, get_args_filter_pack};
use crate::filter::{FilterConfig, filter_args};
use crate::targets::{normalize_src_name, get_executable_names, get_executable_paths, get_main_and_deputy_executable_paths};

/// Holds runtime information for the wrapper.
pub struct Runtime {
    pub src_name: String,
    pub canonical_name: String,
    pub current_executable: String,
    pub input_args: Vec<String>,
    pub target_executable_names: (String, String),
    pub target_executable_paths: (String, String),
    pub target_classification: (ExecutableFamily, ExecutableKind),
    pub main_exe: String,
    pub deputy_exe: String,
    pub final_args: Vec<String>,
    pub expect: String,
}

impl Runtime {
    /// Creates a new Runtime from the Cargo binary name and CLI args.
    /// `src_name` is the `[[bin]] name` from Cargo.toml (via `env!("CARGO_BIN_NAME")`
    /// in `src/main.rs`). `pp` spellings are normalized to `++` here so tool
    /// resolution matches the real compilers (`clang++`, `g++`).
    pub fn new(src_name: String, mut input_args: Vec<String>) -> Self {
        let mut final_args: Vec<String>;
        let canonical_name = normalize_src_name(&src_name);
        // Strip the `-rs` wrapper suffix to get the real tool name (e.g. `clang++-rs` -> `clang++`).
        // The `-rs` suffix is kept on `canonical_name` for PROJECT_SIGNATURE self-detection.
        let src_executable = canonical_name.strip_suffix("-rs").unwrap_or(&canonical_name).to_string();
        trace!("Src name: {} -> canonical: {} -> tool: {}", src_name, canonical_name, src_executable);

        let current_executable = std::env::current_exe().unwrap().file_name().unwrap().to_str().unwrap().to_string();
        trace!("Current executable: {}", current_executable);

        let target_executable_names: (String, String) = get_executable_names(&src_executable, &mut input_args);
        trace!("Target executable names: {:?}", target_executable_names);

        let target_executable_paths: (String, String) = get_executable_paths(&target_executable_names);
        trace!("Target executable paths: {:?}", target_executable_paths);

        let (main_exe, deputy_exe): (String, String) = get_main_and_deputy_executable_paths(&target_executable_paths);
        info!("Main exe: {}, Deputy exe: {}", main_exe, deputy_exe);

        let target_classification: (ExecutableFamily, ExecutableKind) = get_target_classification(&target_executable_names.1);
        debug!("Target classification: {:?}", target_classification);

        // Check to see if passthrough mode is enabled to skip args processing
        if std::env::var("WRAPPER_ENABLE_PASSTHROUGH_MODE").is_ok() {
            debug!("Passthrough mode enabled - skipping all argument processing");
            final_args = input_args.clone();
        } else {
            debug!("Passthrough mode not enabled - processing arguments");
            let (bad_flags, swap_pairs, extra_flags) = get_args_filter_pack(target_classification);
            final_args = filter_args(input_args.clone(), &bad_flags, &swap_pairs, &extra_flags.to_string(), &FilterConfig::from_env(), target_classification.0);
        }

        if deputy_exe != NONE_KEYWORD {
            final_args.insert(0, deputy_exe.clone())
        }
        let expect: String = main_exe.clone() + " died";

        Runtime {
            src_name,
            canonical_name,
            current_executable,
            input_args,
            target_executable_names,
            target_executable_paths,
            target_classification,
            main_exe,
            deputy_exe,
            final_args,
            expect,
        }
    }

    pub fn print_info(&self) {
        debug!("Src Name: {}", self.src_name);
        debug!("Canonical Name: {}", self.canonical_name);
        debug!("Current Executable: {}", self.current_executable);
        warn!("Input Args: {:?}", self.input_args);
        debug!("Target Executable Names: {:?}", self.target_executable_names);
        info!("Target Classification: {:?}", self.target_classification);
        info!("Target Executable Paths: {:?}", self.target_executable_paths);
        warn!("Final Args: {:?}", self.final_args);
        trace!("Expect: {}", self.expect);
    }
}