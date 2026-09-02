// Module declarations
mod constants;
mod flags;
mod classification;
mod filter;
mod runtime;
mod logger;
mod usage;
mod executable;

// Re-export public API
pub use classification::{ExecutableFamily, ExecutableKind, get_target_classification, get_args_filter_pack};
pub use filter::{FilterConfig, filter_args, apply_filter, find_options_end, is_pure_link_step};
pub use runtime::Runtime;
pub use logger::init_logger;
pub use usage::print_usage;
pub use executable::{get_executable_names, get_executable_paths, get_main_and_deputy_executable_paths};

// Re-export constants
pub use constants::{
    LLVM_PATH_VS, MSVC_PATH, LLVM_PATH, GCC_PATH, WRAPPER_PATH,
    PATHS, WRAPPER_KEYWORDS, SELF_WRAPPER_SIGNATURE,
    COMPILER_KEYWORDS, LINKER_KEYWORDS,
    LLVM_KEYWORDS, MSVC_KEYWORDS, GCC_KEYWORDS,
    ARGS_CHAR_LIMIT, UNKNOWN_KEYWORD, BAD_MATCH_MESSAGE, RESPONSE_FILE_NAME,
    LLVM_COMPILER_EXTRA_FLAGS, LLVM_LINKER_EXTRA_FLAGS,
    MSVC_COMPILER_EXTRA_FLAGS, MSVC_LINKER_EXTRA_FLAGS,
    GCC_COMPILER_EXTRA_FLAGS, GCC_LINKER_EXTRA_FLAGS,
    LLVM_COMPILER_SWAP_PAIRS, LLVM_LINKER_SWAP_PAIRS,
    MSVC_COMPILER_SWAP_PAIRS, MSVC_LINKER_SWAP_PAIRS,
    GCC_COMPILER_SWAP_PAIRS, GCC_LINKER_SWAP_PAIRS,
    LLVM_COMPILER_BAD_FLAGS, LLVM_LINKER_BAD_FLAGS,
    MSVC_COMPILER_BAD_FLAGS, MSVC_LINKER_BAD_FLAGS,
    GCC_COMPILER_BAD_FLAGS, GCC_LINKER_BAD_FLAGS,
    COMMON_SPLIT_FLAGS,
};

#[cfg(test)]
mod tests;
