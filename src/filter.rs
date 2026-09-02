use log::{trace, warn};
use regex::Regex;
use crate::classification::ExecutableFamily;
use crate::constants::ARGS_CHAR_LIMIT;
use crate::flags::flags_with_value;
use std::env;

/// Configuration controlling which filtering steps run. Derived from env vars
/// in `filter_args`; kept as data so the pure logic in `apply_filter` is unit-testable
/// without mutating global process environment (which is unsafe across parallel tests).
#[derive(Default)]
pub struct FilterConfig {
    pub skip_split: bool,            // skip splitting fused /Fd<dir> /Fo<dir> flags
    pub skip_bad: bool,              // skip removing bad flags
    pub skip_swap: bool,             // skip swapping problematic flags
    pub skip_add: bool,              // skip appending extra helpful flags
    pub skip_version_on_empty: bool, // skip auto-adding --version when args is empty
    pub force_response_files: bool,  // always use a response file (regardless of length)
    pub args_char_limit: usize,      // override for the argument character limit
}

impl FilterConfig {
    // Reads the WRAPPER_* environment variables into a FilterConfig.
    pub fn from_env() -> Self {
        let skip_all = env::var("WRAPPER_SKIP_ALL_FLAGS").is_ok();
        FilterConfig {
            // Splitting fused flags is OFF by default; WRAPPER_SPLIT_FLAGS opts it back in.
            skip_split: skip_all || !env::var("WRAPPER_SPLIT_FLAGS").is_ok(),
            skip_bad: skip_all || env::var("WRAPPER_SKIP_BAD_FLAGS").is_ok(),
            skip_swap: skip_all || env::var("WRAPPER_SKIP_SWAP_FLAGS").is_ok(),
            skip_add: skip_all || env::var("WRAPPER_SKIP_ADD_FLAGS").is_ok(),
            skip_version_on_empty: env::var("WRAPPER_SKIP_VERSION_ON_EMPTY").is_ok(),
            force_response_files: env::var("WRAPPER_FORCE_RESPONSE_FILES").is_ok(),
            args_char_limit: FilterConfig::get_args_char_limit(),
        }
    }

    // Resolves the effective character limit, honoring WRAPPER_ARGS_CHAR_LIMIT.
    fn get_args_char_limit() -> usize {
        match env::var("WRAPPER_ARGS_CHAR_LIMIT") {
            Ok(v) => v.trim().parse::<usize>().unwrap_or(ARGS_CHAR_LIMIT),
            Err(_) => ARGS_CHAR_LIMIT,
        }
    }
}

/// Apply the full filter pipeline to the arguments.
///
/// Steps:
/// 1. Split combined flags into individual flags
/// 2. Remove bad flags
/// 3. Swap flags according to the swap pairs
/// 4. Insert extra flags at the appropriate position
pub fn apply_filter(
    args: Vec<String>,
    config: &FilterConfig,
    bad_flags: &Regex,
    swap_pairs: &[(Regex, String)],
    extra_flags: &str,
) -> Vec<String> {
    // Step 1: Split combined flags
    let mut split_args = if config.skip_split {
        args
    } else {
        split_flags(args)
    };
    trace!("After split: {:?}", split_args);

    // Step 2: Remove bad flags
    if !config.skip_bad {
        split_args = split_args
            .into_iter()
            .filter(|a| !bad_flags.is_match(a))
            .collect();
        trace!("After bad flag removal: {:?}", split_args);
    }

    // Step 3: Swap flags
    if !config.skip_swap {
        for (regex, replacement) in swap_pairs {
            split_args = split_args
                .into_iter()
                .map(|a| {
                    if regex.is_match(&a) {
                        trace!("Swapping flag: {} -> {}", a, replacement);
                        replacement.clone()
                    } else {
                        a
                    }
                })
                .collect();
        }
        trace!("After flag swapping: {:?}", split_args);
    }

    // Step 4: Insert extra flags
    if !config.skip_add && !extra_flags.is_empty() {
        let extra: Vec<String> = extra_flags.split_whitespace().map(|s| s.to_string()).collect();
        if !extra.is_empty() {
            // Determine the family from the first arg (if it's a flag)
            let family = ExecutableFamily::LLVM; // Default, will be overridden by caller
            insert_extra_flags(&mut split_args, &extra, family);
        }
    }

    split_args
}

/// Split combined flags into individual flags.
fn split_flags(args: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for arg in args {
        if arg.starts_with('-') || arg.starts_with('/') {
            if is_single_flag(&arg) {
                result.push(arg);
            } else {
                result.extend(split_combined_flags(&arg));
            }
        } else {
            result.push(arg);
        }
    }
    result
}

/// Check if a flag is a single, indivisible flag.
fn is_single_flag(arg: &str) -> bool {
    let single_flags = [
        "-O0", "-O1", "-O2", "-O3", "-Os", "-Oz", "-Ofast",
        "-Wall", "-Wextra", "-Werror", "-Wpedantic",
        "-std=c89", "-std=c99", "-std=c11", "-std=c17", "-std=c23",
        "-std=c++11", "-std=c++14", "-std=c++17", "-std=c++20", "-std=c++23",
        "-std=gnu++11", "-std=gnu++14", "-std=gnu++17", "-std=gnu++20", "-std=gnu++23",
        "-MD", "-MT", "-MF", "-MQ",
        "-c", "-S", "-E",
        "-shared", "-static", "-fPIC",
        "-g", "-g0", "-g1", "-g2", "-g3",
        "-v", "-###", "-help", "--help",
    ];
    single_flags.contains(&arg)
}

/// Split a combined flag into individual flags.
/// For example, `/Fdsome\\target\\directory` becomes `["/Fd", "some\\target\\directory"]`.
fn split_combined_flags(arg: &str) -> Vec<String> {
    // Known flag prefixes that can be combined with their values.
    // Note: -D and /D are NOT included because the define name is always attached
    // to the prefix (e.g., -D_MBCS is a single flag, not -D + _MBCS).
    let prefixes = [
        "/Fd", "/Fo", "/Fp", "/Fe", "/Fa", "/Fm", "/FR", "/FU",
        "-Fd", "-Fo", "-Fp", "-Fe", "-Fa", "-Fm", "-FR", "-FU",
        "/I", "/L", "/l",
        "-I", "-L", "-l",
    ];

    for prefix in &prefixes {
        if arg.starts_with(prefix) && arg.len() > prefix.len() {
            let value = &arg[prefix.len()..];
            return vec![prefix.to_string(), value.to_string()];
        }
    }

    // No known prefix found, return as-is
    vec![arg.to_string()]
}

/// Check if an argument is a source file.
fn is_source_arg(arg: &str) -> bool {
    let source_extensions = [".c", ".cc", ".cpp", ".cxx", ".c++", ".h", ".hpp", ".hxx", ".s", ".S", ".asm"];
    source_extensions.iter().any(|ext| arg.ends_with(ext))
}

/// Insert extra flags at the appropriate position in the argument list.
fn splice_at(args: &mut Vec<String>, pos: usize, insert: &[String]) {
    let mut drain = args.split_off(pos);
    args.extend_from_slice(insert);
    args.append(&mut drain);
}

fn insert_extra_flags(args: &mut Vec<String>, extra: &[String], family: ExecutableFamily) {
    let pos = find_options_end(args, family);
    splice_at(args, pos, extra);
}

/// Find the index where options end and positional arguments begin.
pub fn find_options_end(args: &[String], family: ExecutableFamily) -> usize {
    let flag_values = flags_with_value(family);
    let n = args.len();
    let mut i = 0;

    while i < n {
        let arg = &args[i];
        let takes_value = flag_values.iter().any(|f| arg == f);

        if takes_value {
            if i + 1 < n {
                i += 2;
            } else {
                i += 1;
            }
        } else if arg.starts_with('-') || arg.starts_with('/') {
            // Defensive heuristic: warn about possible unrecognized flag+value pairs
            if i + 1 < n {
                let next = &args[i + 1];
                if !next.starts_with('-') && !next.starts_with('/') && !is_source_arg(next) {
                    warn!(
                        "Possible unrecognized flag+value pair: {:?} {:?}. If {:?} takes a value, add it to FLAGS_WITH_VALUE.",
                        arg, next, arg
                    );
                }
            }
            i += 1;
        } else {
            break;
        }
    }

    i
}

/// Check if the command is a pure link step (no source files).
pub fn is_pure_link_step(args: &[String]) -> bool {
    let has_object = args.iter().any(|a| {
        let lower = a.to_lowercase();
        lower.ends_with(".obj") || lower.ends_with(".o") || lower.ends_with(".lib") || lower.ends_with(".a")
    });
    let has_source = args.iter().any(|a| is_source_arg(a));
    has_object && !has_source
}

/// Filter and transform command-line arguments for the target executable.
pub fn filter_args(args: Vec<String>, bad_flags: &Regex, swap_pairs: &[(Regex, String)], extra_flags: &str, config: &FilterConfig) -> Vec<String> {
    apply_filter(args, config, bad_flags, swap_pairs, extra_flags)
}
