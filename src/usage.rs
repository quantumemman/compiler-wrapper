use std::env;

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

/// Prints the wrapper usage/help message when `WRAPPER_OPTIONS` or `WRAPPER_HELP` is set.
/// Returns `true` if the help message was shown (so the caller can exit), `false` otherwise.
pub fn print_usage() -> bool {
    if !(env::var("WRAPPER_OPTIONS").is_ok() || env::var("WRAPPER_HELP").is_ok()) {
        return false;
    }

    const INNER: usize = 90; // usable text columns between the two side borders
    const NAME_W: usize = 30; // reserved width for the variable-name column
    let rule = "═".repeat(INNER + 2);

    let mut lines: Vec<String> = Vec::new();

    // ---- Heading ----
    lines.push(box_row("", INNER));
    lines.push(box_row(
        &center_text(
            &paint("WRAPPER — Compiler Argument Wrapper Helper", &format!("{}{}", ansi::BOLD, ansi::CYAN)),
            INNER,
        ),
        INNER,
    ));
    lines.push(box_row(
        &center_text(&paint("Environment variables understood by the wrapper", ansi::DIM), INNER),
        INNER,
    ));
    lines.push(box_row("", INNER));

    // ---- Flags & behaviour ----
    lines.push(section_row("FLAGS & BEHAVIOUR", INNER));
    lines.extend(usage_pair(
        "WRAPPER_PREFER_VS",
        "Prefer VS Studio LLVM executables over ROCm LLVM.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_CLANG_CL_IS_LLVM",
        "Route clang-cl down the LLVM family (the default). Declaring it explicitly makes the intent immune to default changes.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_GCC_IS_LLVM",
        "Route g++/gcc down the LLVM family. Declaring it explicitly makes the intent immune to default changes.",
        NAME_W,
        INNER,
    ));

    // ---- Runtime info ----
    lines.push(section_row("RUNTIME INFO", INNER));
    lines.extend(usage_pair(
        "WRAPPER_OPTIONS",
        "Print this help message and exit.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_HELP",
        "Print this help message and exit.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_LOG_LEVEL",
        "Set the log level (trace, debug, info, warn, error). Default: info.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_LOG_FILE",
        "Path to the log file. Default: <temp_dir>/wrapper_logs/wrapper_<pid>.log.",
        NAME_W,
        INNER,
    ));

    // ---- Filtering ----
    lines.push(section_row("FILTERING", INNER));
    lines.extend(usage_pair(
        "WRAPPER_DISABLE_BAD_FLAGS",
        "Disable bad flag filtering.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_DISABLE_SWAP_PAIRS",
        "Disable flag swapping.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_DISABLE_EXTRA_FLAGS",
        "Disable extra flag insertion.",
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
