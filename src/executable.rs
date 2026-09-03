use std::path::{Path, PathBuf};
use log::{debug, trace};
use crate::constants::{PATHS, UNKNOWN_KEYWORD, EXTERNAL_WRAPPER_KEYWORDS, COMPILER_KEYWORDS, LINKER_KEYWORDS, PROJECT_SIGNATURE};

/// Split the source executable name into (wrapper, tool) pair.
/// For combined forms like "sccache-clang-cl", this returns ("sccache", "clang-cl").
/// For bare wrappers like "sccache", the first input arg is consumed as the tool.
/// For non-wrappers like "clang-cl", this returns ("UNKNOWN", "clang-cl").
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
fn find_executable(executable_name: &str, paths: &[&str]) -> Option<PathBuf> {
    if executable_name != UNKNOWN_KEYWORD {
        if !(PROJECT_SIGNATURE.is_match(&executable_name) || Path::new(executable_name).is_absolute()) {
            for dir in paths {
                let candidate = Path::new(dir).join(executable_name);
                let candidate = if cfg!(windows) {
                    candidate.with_extension("exe")
                } else {
                    candidate
                };
                trace!("Checking candidate path: {:?}", candidate);
                if candidate.exists() && candidate.is_file() {
                    debug!("Found {} in {}", executable_name, dir);
                    return Some(candidate.to_path_buf());
                }
            }
        } else {
            debug!("Executable is absolute or another wrapper, returning as is: {}", executable_name);
            return Some(PathBuf::from(executable_name));
        }
    }
    None
}

/// Resolve the full paths for the wrapper and tool executables.
pub fn get_executable_paths(target_executable_names: &(String, String)) -> (String, String) {
    let wrapper_path: String = find_executable(&target_executable_names.0, &*PATHS)
        .unwrap_or(UNKNOWN_KEYWORD.into())
        .to_str()
        .unwrap_or("")
        .to_string();
    let executable_path: String = find_executable(&target_executable_names.1, &*PATHS)
        .unwrap_or(UNKNOWN_KEYWORD.into())
        .to_str()
        .unwrap_or("")
        .to_string();
    (wrapper_path.replace("\\", "/"), executable_path.replace("\\", "/"))
}

/// Determine the main (wrapper) and deputy (tool) executable paths.
pub fn get_main_and_deputy_executable_paths(target_executable_names: &(String, String)) -> (String, String) {
    let deputy_executable: String = target_executable_names.1.clone();
    let main_executable: String = if target_executable_names.0 != UNKNOWN_KEYWORD {
        target_executable_names.0.clone()
    } else {
        deputy_executable.clone()
    };
    (main_executable, deputy_executable)
}
