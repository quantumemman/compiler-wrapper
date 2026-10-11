// Module declarations
mod logger;
mod constants;
mod classify;
mod filter;
mod parser;
mod runtime;
mod targets;
mod usage;

// Re-export public API
pub use logger::init_logger;
pub use classify::{ExecutableFamily, ExecutableKind, get_target_classification, get_args_filter_pack};
pub use filter::{FilterConfig, filter_args, apply_filter};
pub use parser::{parse_args, locate_last_flag, find_options_end, flag_takes_separate_value, embedded_flag_value, is_source_arg, LocatedFlag};
pub use runtime::Runtime;
pub use targets::{normalize_src_name, get_executable_names, get_executable_paths, get_main_and_deputy_executable_paths};
pub use usage::{print_usage, help_message, version_message};

// Re-export constants
pub use constants::{
    VS_LLVM_PATH, MSVC_PATH, LLVM_PATH, GCC_PATH, WRAPPER_PATH,
    PATHS, EXTERNAL_WRAPPER_KEYWORDS, PROJECT_SIGNATURE,
    ARGS_CHAR_LIMIT, BAD_MATCH_MESSAGE, RESPONSE_FILE_NAME,
    COMPILER_KEYWORDS, LINKER_KEYWORDS, SPLIT_FUSED_FLAGS, FIX_FLAG_PREFIXES,
    LLVM_KEYWORDS, MSVC_KEYWORDS, GCC_KEYWORDS, UNKNOWN_KEYWORD,
    LLVM_COMPILER_EXTRA_FLAGS, LLVM_LINKER_EXTRA_FLAGS,
    MSVC_COMPILER_EXTRA_FLAGS, MSVC_LINKER_EXTRA_FLAGS,
    GCC_COMPILER_EXTRA_FLAGS, GCC_LINKER_EXTRA_FLAGS,
    LLVM_COMPILER_SWAP_PAIRS, LLVM_LINKER_SWAP_PAIRS,
    MSVC_COMPILER_SWAP_PAIRS, MSVC_LINKER_SWAP_PAIRS,
    GCC_COMPILER_SWAP_PAIRS, GCC_LINKER_SWAP_PAIRS,
    LLVM_COMPILER_BAD_FLAGS, LLVM_LINKER_BAD_FLAGS,
    MSVC_COMPILER_BAD_FLAGS, MSVC_LINKER_BAD_FLAGS,
    GCC_COMPILER_BAD_FLAGS, GCC_LINKER_BAD_FLAGS,
    CLI_HELP_FLAGS, CLI_VERSION_FLAGS
};

#[cfg(test)]
mod tests;
