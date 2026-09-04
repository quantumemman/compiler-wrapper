use crate::constants::{LLVM_KEYWORDS, MSVC_KEYWORDS, GCC_KEYWORDS, COMPILER_KEYWORDS, LINKER_KEYWORDS};
use crate::constants::{LLVM_COMPILER_BAD_FLAGS, MSVC_COMPILER_BAD_FLAGS, GCC_COMPILER_BAD_FLAGS};
use crate::constants::{LLVM_LINKER_BAD_FLAGS, MSVC_LINKER_BAD_FLAGS, GCC_LINKER_BAD_FLAGS};
use crate::constants::{LLVM_COMPILER_SWAP_PAIRS, MSVC_COMPILER_SWAP_PAIRS, GCC_COMPILER_SWAP_PAIRS};
use crate::constants::{LLVM_LINKER_SWAP_PAIRS, MSVC_LINKER_SWAP_PAIRS, GCC_LINKER_SWAP_PAIRS};
use crate::constants::{LLVM_COMPILER_EXTRA_FLAGS, MSVC_COMPILER_EXTRA_FLAGS, GCC_COMPILER_EXTRA_FLAGS};
use crate::constants::{LLVM_LINKER_EXTRA_FLAGS, MSVC_LINKER_EXTRA_FLAGS, GCC_LINKER_EXTRA_FLAGS};
use regex::Regex;
use std::env;

/// The compiler/linker family an executable belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableFamily {
    LLVM,
    MSVC,
    GCC,
    Unknown,
}

/// The role of the executable — compiler or linker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableKind {
    Compiler,
    Linker,
    Unknown,
}

/// Classify a executable path into a (family, kind) tuple.
pub fn get_target_classification(executable_name: &str) -> (ExecutableFamily, ExecutableKind) {
    // treat clang-cl as MSVC family unless requested then treat as LLVM
    let family = if executable_name.contains("clang-cl") && !env::var("WRAPPER_CLANG_CL_IS_LLVM").is_ok() {
        ExecutableFamily::MSVC
    } else if LLVM_KEYWORDS.is_match(&executable_name) {
        ExecutableFamily::LLVM
    } else if MSVC_KEYWORDS.is_match(&executable_name) {
        ExecutableFamily::MSVC
    } else if GCC_KEYWORDS.is_match(&executable_name) {
        ExecutableFamily::GCC
    } else {
        ExecutableFamily::Unknown
    };

    let kind = if COMPILER_KEYWORDS.is_match(&executable_name) {
        ExecutableKind::Compiler
    } else if LINKER_KEYWORDS.is_match(&executable_name) {
        ExecutableKind::Linker
    } else {
        ExecutableKind::Unknown
    };

    (family, kind)
}

/// Returns the filter pack (bad flags regex, swap pairs, extra flags string)
/// for a given classification.
pub fn get_args_filter_pack(classification: (ExecutableFamily, ExecutableKind)) -> (Regex, Vec<(Regex, String)>, String) {
    match classification {
        (ExecutableFamily::LLVM, ExecutableKind::Compiler) => (LLVM_COMPILER_BAD_FLAGS.clone(), LLVM_COMPILER_SWAP_PAIRS.clone(), LLVM_COMPILER_EXTRA_FLAGS.to_string()),
        (ExecutableFamily::MSVC, ExecutableKind::Compiler) => (MSVC_COMPILER_BAD_FLAGS.clone(), MSVC_COMPILER_SWAP_PAIRS.clone(), MSVC_COMPILER_EXTRA_FLAGS.to_string()),
        (ExecutableFamily::GCC, ExecutableKind::Compiler) => (GCC_COMPILER_BAD_FLAGS.clone(), GCC_COMPILER_SWAP_PAIRS.clone(), GCC_COMPILER_EXTRA_FLAGS.to_string()),
        (ExecutableFamily::LLVM, ExecutableKind::Linker) => (LLVM_LINKER_BAD_FLAGS.clone(), LLVM_LINKER_SWAP_PAIRS.clone(), LLVM_LINKER_EXTRA_FLAGS.to_string()),
        (ExecutableFamily::MSVC, ExecutableKind::Linker) => (MSVC_LINKER_BAD_FLAGS.clone(), MSVC_LINKER_SWAP_PAIRS.clone(), MSVC_LINKER_EXTRA_FLAGS.to_string()),
        (ExecutableFamily::GCC, ExecutableKind::Linker) => (GCC_LINKER_BAD_FLAGS.clone(), GCC_LINKER_SWAP_PAIRS.clone(), GCC_LINKER_EXTRA_FLAGS.to_string()),
        _ => panic!("Invalid executable family and kind"), // Add new cases when triggered
    }
}
