use std::env;
use std::path::Path;
use log::{debug, info, trace, warn};
use crate::filter::{FilterConfig, filter_args};
use crate::constants::{UNKNOWN_KEYWORD};
use crate::executable::{get_executable_names, get_executable_paths, get_main_and_deputy_executable_paths};
use crate::classification::{ExecutableFamily, ExecutableKind, get_target_classification, get_args_filter_pack};

/// Holds runtime information for the wrapper.
pub struct Runtime {
    pub src_file: String,
    pub src_executable: String,
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
    /// Creates a new Runtime from the given arguments.
    pub fn new(src_file: String, mut input_args: Vec<String>) -> Self {
        let mut final_args: Vec<String>;
        let src_executable = Path::new(&src_file).file_name().unwrap().to_str().unwrap().to_string().trim_end_matches(".rs").to_string();
        trace!("Src executable: {}", src_executable);

        let target_executable_names: (String, String) = get_executable_names(&src_executable, &mut input_args);
        trace!("Target executable names: {:?}", target_executable_names);

        let target_executable_paths: (String, String) = get_executable_paths(&target_executable_names);
        trace!("Target executable paths: {:?}", target_executable_paths);

        let (main_exe, deputy_exe): (String, String) = get_main_and_deputy_executable_paths(&target_executable_paths);
        info!("Main exe: {}, Deputy exe: {}", main_exe, deputy_exe);

        let target_classification: (ExecutableFamily, ExecutableKind) = get_target_classification(&deputy_exe);
        debug!("Target classification: {:?}", target_classification);

        // Check to see if passthrough mode is enabled to skip args processing
        if env::var("WRAPPER_ENABLE_PASSTHROUGH").is_ok() {
            debug!("Passthrough mode enabled - skipping all argument processing");
            final_args = input_args.clone();
        } else {
            debug!("Passthrough mode not enabled - processing arguments");
            let (bad_flags, swap_pairs, extra_flags) = get_args_filter_pack(target_classification);
            final_args = filter_args(input_args.clone(), &bad_flags, &swap_pairs, &extra_flags.to_string(), &FilterConfig::from_env(), target_classification.0);
        }

        if target_executable_names.0 != UNKNOWN_KEYWORD {
            final_args.insert(0, deputy_exe.clone())
        }
        let expect: String = deputy_exe.clone() + " died";

        Runtime {
            src_file,
            src_executable,
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
        debug!("Src File: {}", self.src_file);
        debug!("Src Executable: {}", self.src_executable);
        info!("Input Args: {:?}", self.input_args);
        debug!("Target Executable Names: {:?}", self.target_executable_names);
        warn!("Target Classification: {:?}", self.target_classification);
        info!("Target Executable Paths: {:?}", self.target_executable_paths);
        warn!("Final Args: {:?}", self.final_args);
        debug!("Expect: {}", self.expect);
    }
}