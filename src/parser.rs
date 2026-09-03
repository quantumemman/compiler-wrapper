//! Family-aware command-line parsing for compiler/linker invocations.
//!
//! This layer sits on top of the [`zccache_depgraph`] parsers
//! ([`zccache_depgraph::args::parse_gnu_args`] for GNU/Clang,
//! [`zccache_depgraph::msvc_args::parse_msvc_args`] for MSVC) and adds what
//! their parsed result does not expose directly:
//!
//! * which flag consumes the *next* token as its value,
//! * where the options region ends (where extra flags get spliced in),
//! * where the last flag lives.
//!
//! Knowing which flags consume a value is what lets extra flags be inserted
//! without landing between a flag and its operand.

use std::path::Path;

use log::warn;
use zccache_depgraph::args::{ParsedArgs, parse_gnu_args};
use zccache_depgraph::msvc_args::parse_msvc_args;

use crate::classification::ExecutableFamily;

/// Probe token fed to a parser to see whether a flag swallows the next
/// argument. Its value never matters, only whether it ends up as a
/// source-file candidate (which is when `source_file` becomes non-empty).
const VALUE_PROBE: &str = "ZzzValueProbe";

/// Parse `args` (everything after the compiler executable) with the parser
/// matching `family`: GNU/Clang-style for LLVM/GCC/Unknown, MSVC-style for
/// MSVC.
pub fn parse_args(args: &[String], cwd: &Path, family: ExecutableFamily) -> ParsedArgs {
    match family {
        ExecutableFamily::MSVC => parse_msvc_args(args, cwd),
        _ => parse_gnu_args(args, cwd),
    }
}

/// True when `flag` consumes the *following* token as its value (e.g.
/// `-o out.o`, `-I include`). Fused forms (`-O2`, `-DDEBUG`, `-Fodir`) return
/// false: they hold their value inside the single token.
///
/// The [`zccache_depgraph`] parser is the sole authority: feeding it `flag`
/// plus a probe token, the flag takes a value iff the probe token is swallowed
/// and never becomes a source-file candidate. No hardcoded supplement lists —
/// if the parser does not know a flag, the defensive warning in
/// [`find_options_end`] fires so it can be added to the parser.
pub fn flag_takes_separate_value(flag: &str, family: ExecutableFamily) -> bool {
    flag_takes_separate_value_per_parser(flag, family)
}

/// Ask the family's parser whether `flag` swallows [`VALUE_PROBE`].
///
/// Path resolution never matters here: whichever way the probe token is
/// resolved, it either stays a separate source candidate (non-empty
/// `source_file`) or is consumed as a value (empty `source_file`).
fn flag_takes_separate_value_per_parser(flag: &str, family: ExecutableFamily) -> bool {
    let probe = [flag.to_string(), VALUE_PROBE.to_string()];
    let source_has_no_probe = match family {
        ExecutableFamily::MSVC => parse_msvc_args(&probe, Path::new("."))
            .source_file
            .as_path()
            .as_os_str()
            .is_empty(),
        _ => parse_gnu_args(&probe, Path::new("."))
            .source_file
            .as_path()
            .as_os_str()
            .is_empty(),
    };
    source_has_no_probe
}

/// Find the index where options end and positional arguments begin.
///
/// The scan skips flags together with their consumed values, stopping at the
/// first positional argument and at the `--` end-of-options marker
/// (everything from `--` onward is positional).
pub fn find_options_end(args: &[String], family: ExecutableFamily) -> usize {
    let n = args.len();
    let mut i = 0;

    while i < n {
        let arg = &args[i];

        // `--` ends option parsing for GNU/Clang: leave it and the rest alone.
        if arg == "--" {
            break;
        }

        if flag_takes_separate_value(arg, family) {
            if i + 1 < n {
                i += 2;
            } else {
                i += 1;
            }
        } else if arg.starts_with('-') || arg.starts_with('/') {
            // Defensive heuristic: warn about a possible unrecognized flag+value pair.
            if i + 1 < n {
                let next = &args[i + 1];
                if !next.starts_with('-') && !next.starts_with('/') && !is_source_arg(next) {
                    warn!(
                        "Possible unrecognized flag+value pair: {:?} {:?}. If {:?} takes a value, add it to the zccache_depgraph parser.",
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

/// A flag located in a command line, with its value.
pub struct LocatedFlag {
    /// 0-based index of the flag token.
    pub flag_index: usize,
    /// 0-based index of a separate value token the flag consumes
    /// (e.g. `main.o` for `-o main.o`).
    pub value_index: Option<usize>,
    /// The value embedded in the flag token itself
    /// (e.g. `c++17` in `-std=c++17`, `DEBUG` in `-DDEBUG`).
    pub embedded_value: Option<String>,
}

/// Locate the last flag in a command line, family-aware.
///
/// Scans from the end. The `--` end-of-options marker and everything after it
/// are treated as positional, and a lone `-` (stdin/stdout) is not a flag.
pub fn locate_last_flag(args: &[String], family: ExecutableFamily) -> Option<LocatedFlag> {
    // `--` ends option parsing: it and everything after it are positional.
    let option_end = args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(args.len());

    for flag_index in (0..option_end).rev() {
        let flag = &args[flag_index];

        if flag == "-" || !(flag.starts_with('-') || flag.starts_with('/')) {
            continue;
        }

        // A flag such as `-o` or `/Fo` consumes the *next* token as its value,
        // as long as that token is not itself a flag.
        let value_index = if flag_takes_separate_value(flag, family)
            && args
                .get(flag_index + 1)
                .is_some_and(|next| !(next.starts_with('-') || next.starts_with('/')))
        {
            Some(flag_index + 1)
        } else {
            None
        };

        // Otherwise the value may be embedded in the flag token
        // (`-std=c++17`, `-DDEBUG`, `/Foout.obj`).
        let embedded_value = if value_index.is_none() {
            embedded_flag_value(flag, family)
        } else {
            None
        };

        return Some(LocatedFlag {
            flag_index,
            value_index,
            embedded_value,
        });
    }

    None
}

/// Extract a value embedded in a flag token.
///
/// The [`zccache_depgraph`] parser is the authority: parse the single flag
/// token and return whatever value the parser extracted from it. GNU/Clang
/// `-flag=value` style is handled by the parser directly.
pub fn embedded_flag_value(flag: &str, family: ExecutableFamily) -> Option<String> {
    let parsed = parse_args(&[flag.to_string()], Path::new("."), family);

    // `-D<value>` / `/D<value>` / `-U<value>` / `/U<value>`
    if let Some(v) = parsed.defines.first() {
        return Some(v.clone());
    }
    if let Some(v) = parsed.undefines.first() {
        return Some(v.clone());
    }

    // `-I<path>` / `/I<path>` and other include-search paths
    if let Some(p) = parsed.include_search.iquote.first() {
        return Some(p.to_string_lossy().into_owned());
    }
    if let Some(p) = parsed.include_search.user.first() {
        return Some(p.to_string_lossy().into_owned());
    }
    if let Some(p) = parsed.include_search.system.first() {
        return Some(p.to_string_lossy().into_owned());
    }
    if let Some(p) = parsed.include_search.after.first() {
        return Some(p.to_string_lossy().into_owned());
    }

    // `-o <path>` / `/Fo<path>` / etc.
    if let Some(p) = &parsed.output_file {
        let s = p.to_string_lossy().into_owned();
        if !s.is_empty() {
            return Some(s);
        }
    }

    None
}

/// Check if an argument looks like a source file.
pub fn is_source_arg(arg: &str) -> bool {
    let source_extensions = [
        ".c", ".cc", ".cpp", ".cxx", ".c++", ".h", ".hpp", ".hxx", ".s", ".S", ".asm",
    ];
    source_extensions.iter().any(|ext| arg.ends_with(ext))
}
