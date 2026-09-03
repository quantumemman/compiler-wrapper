use log::trace;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use crate::parser::{find_options_end, locate_last_flag};
use crate::classification::ExecutableFamily;
use crate::constants::{ARGS_CHAR_LIMIT, RESPONSE_FILE_NAME, COMMON_SPLIT_FLAGS};

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
    family: ExecutableFamily,
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
            insert_extra_flags(&mut split_args, &extra, family);
        }
    }
    trace!("After extra flags: {:?}", split_args);

    // Step 5: Response file emission
    split_args = maybe_emit_response_file(split_args, config);

    split_args
}

/// Split fused flags into individual flags using [`COMMON_SPLIT_FLAGS`].
/// For example, `/Fdsome\\target\\directory` becomes `["/Fd", "some\\target\\directory"]`.
fn split_flags(args: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for arg in args {
        if arg.starts_with('-') || arg.starts_with('/') {
            if let Some(split) = split_combined_flag(&arg) {
                result.extend(split);
            } else {
                result.push(arg);
            }
        } else {
            result.push(arg);
        }
    }
    result
}

/// Split a fused flag into its prefix and value using [`COMMON_SPLIT_FLAGS`].
/// Returns `None` if the flag does not match any known prefix.
fn split_combined_flag(arg: &str) -> Option<Vec<String>> {
    let caps = COMMON_SPLIT_FLAGS.captures(arg)?;
    let prefix = caps.get(0)?.as_str();
    if prefix.len() < arg.len() {
        let value = &arg[prefix.len()..];
        return Some(vec![prefix.to_string(), value.to_string()]);
    }
    None
}

/// Insert extra flags at the appropriate position in the argument list.
fn splice_at(args: &mut Vec<String>, pos: usize, insert: &[String]) {
    let mut drain = args.split_off(pos);
    args.extend_from_slice(insert);
    args.append(&mut drain);
}

fn insert_extra_flags(args: &mut Vec<String>, extra: &[String], family: ExecutableFamily) {
    // Use locate_last_flag (scanning from the end) to find the last flag, then
    // insert the extra flags immediately before it. This keeps the injected
    // flags inside the options region (before any `--` marker or source file)
    // while ensuring the user's last flag stays at the end of options — e.g.
    // `gcc -c main.c` becomes `gcc EXTRA -c main.c`, not `gcc -c EXTRA main.c`.
    // Falls back to find_options_end (scanning from the front) only when the
    // command has no flags at all.
    let pos = locate_last_flag(args, family)
        .map(|located| located.flag_index)
        .unwrap_or_else(|| find_options_end(args, family));
    splice_at(args, pos, extra);
}

/// If the joined arguments exceed the configured limit (or force_response_files
/// is set), write them to a response file in the temp directory and collapse
/// the argument list to a single `@file` argument. If any argument already
/// starts with `@`, the list is passed through untouched.
fn maybe_emit_response_file(args: Vec<String>, config: &FilterConfig) -> Vec<String> {
    // If any arg already starts with `@`, pass through untouched
    if args.iter().any(|a| a.starts_with('@')) {
        return args;
    }

    // Calculate total character length of joined arguments
    let total_len: usize = args.iter().map(|a| a.len() + 1).sum::<usize>().saturating_sub(1);

    // Determine if we need a response file:
    // - force_response_files effectively sets the limit to 1 (need at least 1 non-compiler arg)
    // - otherwise check against the configured char limit
    let needs_response_file = if config.force_response_files {
        args.len() > 1
    } else {
        total_len > config.args_char_limit
    };

    if !needs_response_file {
        return args;
    }

    // Build the response file path in the system temp directory
    let pid = std::process::id();
    let rsp_name = RESPONSE_FILE_NAME.replace("<pid>", &pid.to_string());
    let rsp_path: PathBuf = std::env::temp_dir().join(&rsp_name);

    // Write each argument on its own line
    if let Ok(mut file) = File::create(&rsp_path) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                let _ = writeln!(file);
            }
            let _ = write!(file, "{}", arg);
        }
        let _ = file.flush();
        trace!("Wrote response file: {:?}", rsp_path);

        // Collapse to a single `@file` argument
        vec![format!("@{}", rsp_path.display())]
    } else {
        // If we can't create the response file, pass through untouched
        trace!("Failed to create response file: {:?}", rsp_path);
        args
    }
}

/// Filter and transform command-line arguments for the target executable.
pub fn filter_args(
    args: Vec<String>,
    bad_flags: &Regex,
    swap_pairs: &[(Regex, String)],
    extra_flags: &str,
    config: &FilterConfig,
    family: ExecutableFamily,
) -> Vec<String> {
    apply_filter(args, config, bad_flags, swap_pairs, extra_flags, family)
}
