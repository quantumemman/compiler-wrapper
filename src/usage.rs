use std::env;
use crate::constants::{NONE_KEYWORD, ARGS_CHAR_LIMIT, CLI_HELP_FLAGS, CLI_VERSION_FLAGS};
use crate::runtime::{Runtime};

/// Package version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ANSI color codes for terminal output.
pub mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    pub const YELLOW: &str = "\x1b[33m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
}

/// Apply ANSI color codes to a string.
fn paint(s: &str, color: &str) -> String {
    format!("{}{}{}", color, s, ansi::RESET)
}

/// Get the visible length of a string (ignoring ANSI escape codes).
fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

/// Wrap text to a given width, breaking at word boundaries.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_width = visible_len(word);
        if current_width + word_width + 1 > width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }
        if !current_line.is_empty() {
            current_line.push(' ');
            current_width += 1;
        }
        current_line.push_str(word);
        current_width += word_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Center text within a given width.
fn center_text(s: &str, width: usize) -> String {
    let len = visible_len(s);
    if len >= width {
        return s.to_string();
    }
    let pad = width - len;
    let left = pad / 2;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(pad - left))
}


/// Build the `• name  description` rows, wrapping long descriptions so that
/// continuation lines align under the description column.
fn usage_pair(name: &str, desc: &str, name_w: usize, inner: usize) -> Vec<String> {
    let bullet = paint("  • ", ansi::MAGENTA);
    let key = paint(name, &format!("{}{}", ansi::BOLD, ansi::CYAN));
    let field_w = 4 + name_w; // bullet + key field
    let field = format!("{bullet}{key}");
    let field_pad = field_w.saturating_sub(visible_len(&field));
    let mut lead = format!("{field}{}", " ".repeat(field_pad));
    lead.push_str("  "); // gap before the description column

    let desc_w = inner.saturating_sub(field_w + 2);
    let indent = " ".repeat(field_w + 2);

    let mut out: Vec<String> = Vec::new();
    for (i, line) in wrap_text(desc, desc_w).into_iter().enumerate() {
        if i == 0 {
            out.push(format!("{lead}{line}"));
        } else {
            out.push(format!("{indent}{line}"));
        }
    }
    out
}

/// A full-width content row inside the box: `║  <content padded>  ║`.
fn box_row(content: &str, inner: usize) -> String {
    let pad = inner.saturating_sub(visible_len(content));
    format!("║ {content}{} ║", " ".repeat(pad))
}

/// A section divider `╠═ title ═══...══╣` with the title colored.
fn section_row(title: &str, inner: usize) -> String {
    let inside = inner + 2; // space between the ╠ and ╣ borders
    let t = format!(" {title} ");
    let rule_units = inside - t.chars().count();
    let left = rule_units / 2;
    let right = rule_units - left;
    let mut s = String::from("╠");
    s.push_str(&"═".repeat(left));
    s.push_str(&paint(&t, &format!("{}{}", ansi::BOLD, ansi::YELLOW)));
    s.push_str(&"═".repeat(right));
    s.push('╣');
    s
}

/// Prints the wrapper usage/help message.
/// Returns `true` if the help message was shown (so the caller can exit), `false` otherwise.
pub fn print_usage(runtime: &Runtime) -> bool {
    const INNER: usize = 96; // usable text columns between the two side borders
    const NAME_W: usize = 32; // reserved width for the variable-name column
    let rule = "═".repeat(INNER + 2);

    let mut lines: Vec<String> = Vec::new();

    // ---- Heading ----
    lines.push(box_row("", INNER));
    lines.push(box_row(
        &center_text(&paint(format!("Usage - wrapper {} v{}", runtime.current_executable, VERSION).as_str(), &format!("{}{}", ansi::BOLD, ansi::CYAN)),
            INNER,
        ),
        INNER,
    ));
    lines.push(box_row(
        &center_text(&paint(format!("-> {}", runtime.main_exe).as_str(), &format!("{}{}", ansi::BOLD, ansi::CYAN)),
            INNER,
        ),
        INNER,
    ));
    lines.push(box_row(
        &center_text(&paint(
            (if runtime.deputy_exe != NONE_KEYWORD { format!("-> {}", runtime.deputy_exe.clone()) }
            else { String::new() }).as_str(), &format!("{}{}", ansi::BOLD, ansi::CYAN)),
            INNER,
        ),
        INNER,
    ));
    lines.push(box_row(
        &center_text(&paint("Environment variables understood by the wrapper", ansi::DIM), INNER),
        INNER,
    ));
    lines.push(box_row("", INNER));

    // ---- Runtime verbosity ----
    lines.push(section_row("RUNTIME VERBOSITY LEVEL", INNER));
    lines.extend(usage_pair(
        "RUST_LOG",
        "Set the log level (trace, debug, info, warn, error). Default: error.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_LOG_FILE",
        format!("Path to log file e.g. /path/to/{}.log.", runtime.current_executable.trim_end_matches(".exe")).as_str(),
        NAME_W,
        INNER,
    ));

    // ---- Executables Discovery Paths ----
    lines.push(section_row("EXECUTABLES DISCOVERY PATHS", INNER));
    lines.extend(usage_pair(
        "WRAPPER_PREFER_VS",
        "Prioritize executables in Visual Studio LLVM Path over custom.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_MSVC_PATH",
        "Path to Microsoft Visual Studio MSVC toolchain.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_VS_LLVM_PATH",
        "Path to Microsoft Visual Studio LLVM toolchain.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_LLVM_PATH",
        "Path to Custom LLVM toolchain.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_GCC_PATH",
        "Path to GNU Compiler Collection toolchain.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_WRAPPER_PATH",
        "Path to wrappers such as sccache and ccache.",
        NAME_W,
        INNER,
    ));

    // ---- Flag Processing (Opt-In)----
    lines.push(section_row("FLAG PROCESSING (OPT-IN)", INNER));
    lines.extend(usage_pair(
        "WRAPPER_CLANG_CL_IS_MSVC",
        "Treat clang-cl as MSVC family (default, no effect).",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_CLANG_CL_IS_LLVM",
        "Treat clang-cl as LLVM family.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_FIX_FLAG_PREFIXES",
        "Enable fixing flag prefixes e.g. /version:0.0 -> -version:0.0",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SPLIT_FUSED_FLAGS",
        "Enable splitting fused flag+value step.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_ARGS_CHAR_LIMIT",
        format!("Override the project's {} character count minimum to use response files.", ARGS_CHAR_LIMIT).as_str(),
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_FORCE_RESPONSE_FILES",
        "Force the use of response files with no character count minimum.",
        NAME_W,
        INNER,
    ));
    
    // ---- Flag Processing (Opt-Out)----
    lines.push(section_row("FLAG PROCESSING (OPT-OUT)", INNER));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_BAD_FLAGS",
        "Skip bad flags removal step.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_SWAP_FLAGS",
        "Skip problematic flags swapping step.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_ADD_FLAGS",
        "Skip extra flags insertion step.",
        NAME_W,
        INNER,
    ));

    // ---- Various Modes ----
    lines.push(section_row("PASSTHROUGH MODE", INNER));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_ALL_FLAGS",
        "Skip all opt-out flag processing i.e. bad, swap, add.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_ENABLE_PASSTHROUGH_MODE",
        "Pass all args directly to the target executable without any flag processing. The wrapper behaves identically to the intended target executable.",
        NAME_W,
        INNER,
    ));

    // ---- CLI Flags ----
    lines.push(section_row("CLI FLAGS", INNER));
    lines.extend(usage_pair(
        &format!("{:?}", CLI_VERSION_FLAGS.join("|")),
        "Print version info and exit.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        &format!("{:?}", CLI_HELP_FLAGS.join("|")),
        "Print this usage message and exit.",
        NAME_W,
        INNER,
    ));

    // ---- Footer ----
    lines.push(box_row("", INNER));
    lines.push(box_row(
        &center_text(&paint("For more information, visit https://github.com/quantumemman/compiler-wrapper", ansi::DIM), INNER),
        INNER,
    ));
    lines.push(box_row("", INNER));

    // Print the box
    let top_border = format!("╔{}╗", rule);
    let bottom_border = format!("╚{}╝", rule);
    println!("{top_border}");
    for line in &lines {
        println!("{line}");
    }
    println!("{bottom_border}");

    true
}

/// Checks if any of the CLI args are help/usage flags.
/// If so, prints the usage message and returns `true`.
pub fn help_message(runtime: &Runtime) -> bool {
    if CLI_HELP_FLAGS.contains(&runtime.input_args[0].as_str()) && !env::var("WRAPPER_ENABLE_PASSTHROUGH_MODE").is_ok() && !env::var("WRAPPER_SKIP_ALL_FLAGS").is_ok() {
        return print_usage(&runtime);  // Print usage if help flags are present and exit
    }
    false
}

/// Checks if any of the CLI args are version flags.
/// If so, prints the version message and returns `true`.
pub fn version_message(runtime: &Runtime) -> bool {
    if CLI_VERSION_FLAGS.contains(&runtime.input_args[0].as_str()) && !env::var("WRAPPER_ENABLE_PASSTHROUGH_MODE").is_ok() && !env::var("WRAPPER_SKIP_ALL_FLAGS").is_ok() {
        println!(">> wrapper {} v{}", runtime.current_executable, VERSION);
        println!("-> {}", runtime.main_exe);
        if runtime.deputy_exe != NONE_KEYWORD { println!("-> {}", runtime.deputy_exe);}
        return true;
    }
    false
}
