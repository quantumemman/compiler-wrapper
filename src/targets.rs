use std::path::{Path};
use log::{debug, trace};
use crate::constants::{PATHS, UNKNOWN_KEYWORD, NONE_KEYWORD, EXTERNAL_WRAPPER_KEYWORDS, 
    COMPILER_KEYWORDS, LINKER_KEYWORDS, PROJECT_SIGNATURE
};

/// Normalize a `CARGO_BIN_NAME` value into the canonical wrapper name. Strips `.exe` if present,
// replaces `pp-rs` with `++-rs`, and strips the `-rs` suffix if present, and returns it.
pub fn normalize_src_name(src_name: &str) -> String {
    let canonical_name = src_name.to_lowercase().strip_suffix(".exe").unwrap_or(src_name)
    .replace("pp-rs", "++-rs").to_string();
    canonical_name
}

/// Split the source executable name into (wrapper, tool) pair.
/// For combined forms like "sccache-clang-cl", this returns ("sccache", "clang-cl").
/// For bare wrappers like "sccache", the first input arg is consumed as the tool.
/// For non-wrappers like "clang++", this returns ("UNKNOWN", "clang++").
/// `src_executable` is the real tool spelling: caller strips the `-rs` wrapper
/// suffix and normalizes `pp` -> `++` before calling.
pub fn get_executable_names(src_executable: &String, input_args: &mut Vec<String>) -> (String, String) {
    let mut wrapper_name = UNKNOWN_KEYWORD.to_string();
    let mut executable_name = wrapper_name.clone();

    if EXTERNAL_WRAPPER_KEYWORDS.is_match(src_executable) {
        if COMPILER_KEYWORDS.is_match(src_executable) || LINKER_KEYWORDS.is_match(src_executable) {
            // Combined form: "<wrapper>-<tool>", e.g. "sccache-clang-cl"
            if let Some((wrapper, tool)) = src_executable.split_once('-') {
                wrapper_name = wrapper.to_string();
                executable_name = tool.to_string();
            }
        } else {
            // Pure wrapper: "sccache" — consume first arg as tool
            wrapper_name = src_executable.clone();
            executable_name = input_args.remove(0);
        }
    } else {
        executable_name = src_executable.clone();
    }
    (wrapper_name, executable_name)
}

/// Find an executable in the given search paths.
fn find_executable(executable_name: &str) -> String {
    if executable_name != UNKNOWN_KEYWORD {
        if !(PROJECT_SIGNATURE.is_match(&executable_name) || Path::new(executable_name).is_absolute()) {
            for dir in std::env::split_paths(&PATHS.clone()) {
                let candidate = Path::new(&dir).join(executable_name);
                let candidate = if cfg!(windows) {
                    candidate.with_extension("exe")
                } else {
                    candidate
                };
                trace!("Checking candidate: {:?}", candidate);
                if candidate.exists() && candidate.is_file() {
                    debug!("Found: {:?}", candidate);
                    return Some(candidate.to_path_buf())
                    .unwrap_or(NONE_KEYWORD.into()).to_str()
                    .unwrap_or(NONE_KEYWORD.into()).to_string();
                }
            }
        } else {
            debug!("Executable is absolute or another wrapper in this project, returning as is: {}", executable_name);
            return executable_name.to_string();
        }
    }
    NONE_KEYWORD.into()
}

/// Resolve the full paths for the wrapper and tool executables.
pub fn get_executable_paths(target_executable_names: &(String, String)) -> (String, String) {
    let wrapper_path: String = find_executable(&target_executable_names.0);
    let executable_path: String = find_executable(&target_executable_names.1);
    (wrapper_path.replace("\\", "/"), executable_path.replace("\\", "/"))
}

/// Determine the main (wrapper) and deputy (tool) executable paths.
pub fn get_main_and_deputy_executable_paths(target_executable_names: &(String, String)) -> (String, String) {
    let mut main_executable = NONE_KEYWORD.to_string();
    let mut deputy_executable = NONE_KEYWORD.to_string();

    if target_executable_names.0 != NONE_KEYWORD {
        main_executable = target_executable_names.0.clone();
        if target_executable_names.1 != NONE_KEYWORD {
            deputy_executable = target_executable_names.1.clone();
        }
    } else if target_executable_names.1 != NONE_KEYWORD {
        main_executable = target_executable_names.1.clone();
    }
    (main_executable, deputy_executable)
}
