// Module declarations
mod constants;
mod executable;
mod classification;
mod filter;
mod parser;
mod runtime;
mod logger;
mod usage;

// Re-export public API
pub use classification::{ExecutableFamily, ExecutableKind, get_target_classification, get_args_filter_pack};
pub use filter::{FilterConfig, filter_args, apply_filter};
pub use parser::{parse_args, locate_last_flag, find_options_end, flag_takes_separate_value, embedded_flag_value, is_source_arg, LocatedFlag};
pub use runtime::Runtime;
pub use logger::init_logger;
pub use usage::{print_usage, check_help_flags, check_version_flags};
pub use executable::{get_executable_names, get_executable_paths, get_main_and_deputy_executable_paths};

// Re-export constants
pub use constants::{
    LLVM_PATH_VS, MSVC_PATH, LLVM_PATH, GCC_PATH, WRAPPER_PATH,
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
    CLI_FLAG_HELP_SHORT, CLI_FLAG_HELP_LONG, CLI_FLAG_USAGE,
    CLI_FLAG_VERSION_SHORT, CLI_FLAG_VERSION_LONG,
};

#[cfg(test)]
mod tests;
