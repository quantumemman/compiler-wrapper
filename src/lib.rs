use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{id};
use std::string::String;
use std::sync::LazyLock; // for LazyLock init
use regex::Regex;
use log::{trace, debug, info, warn}; // import the logging macros. Options include trace, debug, info, warn, error
pub const ARGS_CHAR_LIMIT: usize = 30000;

/////////////////////////////////////////////////////////////////////////////////////////
//                              Executables Search Paths                               //
/////////////////////////////////////////////////////////////////////////////////////////
pub const HIP_PATH: &str = env!("HIP_PATH_");
pub const LLVM_PATH_VS: &str = env!("LLVM_PATH_VS_");
pub const MSVC_PATH: &str = env!("MSVC_PATH_");
pub const LLVM_PATH: &str = env!("LLVM_PATH_");
pub const GCC_PATH: &str = env!("GCC_PATH_");
pub const PY_SCRIPTS_PATH: &str = env!("PY_PATH_");
pub static PATHS: LazyLock<[&str; 6]> = LazyLock::new(|| {
    // If WRAPPER_PREFER_VS is defined, prefer Visual Studio's LLVM toolchain; otherwise prefer ROCm LLVM.
    if env::var("WRAPPER_PREFER_VS").is_ok() {
        info!("Preferring Visual Studio LLVM toolchain");
        [&HIP_PATH, &LLVM_PATH_VS, &MSVC_PATH, &LLVM_PATH, &GCC_PATH, &PY_SCRIPTS_PATH] // Prefer VS LLVM
    } else {
        info!("Preferring ROCm LLVM toolchain");
        [&HIP_PATH, &LLVM_PATH, &LLVM_PATH_VS, &MSVC_PATH, &GCC_PATH, &PY_SCRIPTS_PATH] // Prefer ROCm LLVM
    }
});

/////////////////////////////////////////////////////////////////////////////////////////
//                                Executables Keywords                                 //
/////////////////////////////////////////////////////////////////////////////////////////
pub static wrapperKeywords: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)ccache"#).unwrap());
pub static compilerKeywords: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(clang|hip|cl|gcc|g\+\+)"#).unwrap());
pub static linkerKeywords: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(link|lld|ld)"#).unwrap());

pub static llvmKeywords: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(clang|llvm|lld)"#).unwrap());
pub static msvcKeywords: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(cl|link)"#).unwrap());
pub static gccKeywords: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(gcc|g\+\+|ld)"#).unwrap());

/////////////////////////////////////////////////////////////////////////////////////////
//                  Define Bad Flag Regexes, -/ start, case sensitive                  //
/////////////////////////////////////////////////////////////////////////////////////////
pub static LLVMCompilerBadFlags: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"^[-/](EHsc|permissive-|bigobj|EGR|W3|Wc\+\+11-narrowing|Wincompatible-pointer-types|Wimplicit-function-declaration|Wdeprecated-declarations|Wextern-initializer|Wold-style-cast|Wunused-variable|Wunused-function|Wlogical-op-parentheses|Wunknown-warning-option)$"#).unwrap());
pub static MSVCCompilerBadFlags: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"^[-/](bigobj|GR|Od|W3|Wc\+\+11-narrowing|Wimplicit-function-declaration|Wdeprecated-declarations|Wextern-initializer|Wold-style-cast|Wunused-variable|Wunused-function|Wlogical-op-parentheses|Wunknown-warning-option)$"#).unwrap());
pub static GCCCompilerBadFlags: LazyLock<Regex> = LazyLock::new(||Regex::new(r#""#).unwrap());

pub static LLVMLinkerBadFlags: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[/](INCREMENTAL:NO|MANIFEST:EMBED|MANIFEST:EMBED,ID=2)$"#).unwrap()); // |MANIFESTUAC:NO
pub static MSVCLinkerBadFlags: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[/](INCREMENTAL:NO|MANIFEST:EMBED|MANIFEST:EMBED,ID=2)$"#).unwrap()); // |MANIFESTUAC:NO
pub static GCCLinkerBadFlags: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""#).unwrap());

pub static SplitFlags: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[/](Fd|Fo)"#).unwrap());

//////////////////////////////////////////////////////////////////////////////////////////
//                             Swap Problematic Arguments                               //
//////////////////////////////////////////////////////////////////////////////////////////
pub static LLVMCompilerSwapPairs: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[-]D[/]bigobj$").expect(BadMatchMessage), "".into()),
        (Regex::new(r"^[/]MD\(d\)?$").expect(BadMatchMessage), "-fms-extensions".into()),
        (Regex::new(r"^[/]Zi$").expect(BadMatchMessage), "-g".into()),
        (Regex::new(r"^[/]01$").expect(BadMatchMessage), "-01".into()),
        (Regex::new(r"^[/]02$").expect(BadMatchMessage), "-02".into()),
        (Regex::new(r"^[/]03$").expect(BadMatchMessage), "-03".into()),
        (Regex::new(r"^[/]04$").expect(BadMatchMessage), "-04".into()),
]});

// Swaps: pattern regex → replacement
pub static MSVCCompilerSwapPairs: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]Zi$").expect(BadMatchMessage), "/Z7".into()),
]});

// Swaps: pattern regex → replacement
pub static LLVMLinkerSwapPairs: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BadMatchMessage), "-flto".into()),
]});

// Swaps: pattern regex → replacement
pub static MSVCLinkerSwapPairs: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BadMatchMessage), "-flto".into()),
]});

// Swaps: pattern regex → replacement
pub static GCCCompilerSwapPairs: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]Zi$").expect(BadMatchMessage), "-g".into()),
]});

// Swaps: pattern regex → replacement
pub static GCCLinkerSwapPairs: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BadMatchMessage), "-flto".into()),
]});

//////////////////////////////////////////////////////////////////////////////////////////
//                                 Extraflags to tack on                                //
//////////////////////////////////////////////////////////////////////////////////////////
pub const LLVMCompilerExtraFlags: &str = "-D_USE_MATH_DEFINES -D_CRT_SECURE_NO_WARNINGS -Wno-error -Wno-c++11-narrowing -Wno-incompatible-pointer-types -Wno-implicit-function-declaration -Wno-extern-initializer -Wno-unused-variable -Wno-unused-function -Wno-logical-op-parentheses -Wno-unknown-warning-option -Wno-microsoft-cast -Wno-c++98-compat -Wno-microsoft-include -w";
pub const LLVMLinkerExtraFlags: &str = "/MANIFEST:NO";
pub const MSVCCompilerExtraFlags: &str = "-D_USE_MATH_DEFINES -D_CRT_SECURE_NO_WARNINGS -Wno-errror=implicit-function-declaration -Wno-errror=extern-initializer -Wno-error=unused-variable -Wno-error=unused-function -Wno-error=logical-op-parentheses -w -FS";
pub const MSVCLinkerExtraFlags: &str = "";
pub const GCCCompilerExtraFlags: &str = "-w";
pub const GCCLinkerExtraFlags: &str = "";
pub const BadMatchMessage: &str = "bad match";
pub const ResponseFileName: &str = "response_file.rsp";
pub const ArgsCharLimit: usize = 30000;

#[derive(Debug)]
pub enum EXEFamily {
    LLVM,         // clang, clang++, gcc, g++, lld-link
    MSVC,         // cl, clang-cl, link
    GCC,          // gcc, g++, ld
    UNKNOWN,      // default classification, sccache
}

#[derive(Debug)]
pub enum EXEKind {
    COMPILER,    // compiler, e.g. clang, clang-cl, hipcc, gcc, g++
    LINKER,      // linker, e.g. link, lld-link, ld
    UNKNOWN,     // default classification, this will throw an error
}

pub fn getName(src: &String, args: &Vec<String>) -> (String, String) {
    let baseName = Path::new(&src).file_name().unwrap().to_str().unwrap().to_string().replace(".rs", ".exe");
    let mut wrapperName = "unknown".to_string();
    let mut exeName = baseName.strip_prefix("sccache-").unwrap_or(&baseName).to_string(); // handle sccache-clang.exe, sccache-link.exe, etc.
    if wrapperKeywords.is_match(&baseName) {
        wrapperName = "sccache.exe".to_string();
    }
    if wrapperKeywords.is_match(&exeName) {
        exeName = args.get(0).unwrap_or(&"unknown".to_string()).to_string();
    }
    return (wrapperName, exeName);
}

//////////////////////////////////////////////////////////////////////////////////////////
//                                   Find file                                          //
//////////////////////////////////////////////////////////////////////////////////////////
pub fn getEXE(name: &(String, String)) -> (String, String) {
    let wrapperPath: String = findFile(&name.0, &*PATHS).unwrap_or("unknown".into()).to_str().unwrap_or("").to_string();    
    let exePath: String = findFile(&name.1, &*PATHS).unwrap_or("unknown".into()).to_str().unwrap_or("").to_string();
    return (wrapperPath.replace("\\", "/"), exePath.replace("\\", "/"));
}

fn findFile(name: &str, paths: &[&str]) -> Option<PathBuf> {
    if name != "unknown" {
        let ext = "exe";
        for dir in paths {
            trace!("Searching for {} in {}", name, dir);
            let candidate = Path::new(dir).join(name).with_extension(ext);
            trace!("Checking candidate path: {:?}", candidate);
            if candidate.exists() && candidate.is_file() {
                return Some(candidate.to_path_buf());
            }
        }
    }
    None
}

// Classify the exe kind based on its name using keywords
pub fn getEXEFamily(name: &(String, String)) -> (EXEFamily, EXEKind) {
    let mut exeFamily = EXEFamily::UNKNOWN;
    let mut exeKind = EXEKind::UNKNOWN;
    
    if name.1.contains("clang-cl") {                  // clang-cl is MSVC-compatible
        exeFamily = EXEFamily::MSVC;
    } else if llvmKeywords.is_match(&name.1) {        // LLVM family—add whatever shows up
        exeFamily = EXEFamily::LLVM;
    } else if msvcKeywords.is_match(&name.1) {        // Pure MSVC first—avoid false positives
        exeFamily = EXEFamily::MSVC;
    } else if gccKeywords.is_match(&name.1) {         // GCC family—add whatever shows up
        exeFamily = EXEFamily::GCC;
    }
    
    if compilerKeywords.is_match(&name.1) { // Pure compiler next—avoid false positives
        exeKind = EXEKind::COMPILER;
    } else if linkerKeywords.is_match(&name.1) {   // Pure MSVC first—avoid false positives
        exeKind = EXEKind::LINKER;
    }
    
    return (exeFamily, exeKind);
}

// Regular method
pub fn getFlags(family: &(EXEFamily, EXEKind)) -> (Regex, Vec<(Regex, String)>, &str) {
    match family {
        (EXEFamily::LLVM, EXEKind::COMPILER) => (LLVMCompilerBadFlags.clone(), LLVMCompilerSwapPairs.clone(), LLVMCompilerExtraFlags),
        (EXEFamily::MSVC, EXEKind::COMPILER) => (MSVCCompilerBadFlags.clone(), MSVCCompilerSwapPairs.clone(), MSVCCompilerExtraFlags),
        (EXEFamily::GCC, EXEKind::COMPILER) => (GCCCompilerBadFlags.clone(), GCCCompilerSwapPairs.clone(), GCCCompilerExtraFlags),
        (EXEFamily::LLVM, EXEKind::LINKER) => (LLVMLinkerBadFlags.clone(), LLVMLinkerSwapPairs.clone(), LLVMLinkerExtraFlags),
        (EXEFamily::MSVC, EXEKind::LINKER) => (MSVCLinkerBadFlags.clone(), MSVCLinkerSwapPairs.clone(), MSVCLinkerExtraFlags),
        (EXEFamily::GCC, EXEKind::LINKER) => (GCCLinkerBadFlags.clone(), GCCLinkerSwapPairs.clone(), GCCLinkerExtraFlags),
        _ => panic!("Invalid family and kind"), // Add this later if needed
    }
}

/////////////////////////////////////////////////////////////////////////////////////////
//                                   Filter Arguments                                  //
/////////////////////////////////////////////////////////////////////////////////////////
// Per-call configuration controlling which filtering steps run. Derived from env vars
// in `filterArgs`; kept as data so the pure logic in `applyFilter` is unit-testable
// without mutating global process environment (which is unsafe across parallel tests).
#[derive(Default)]
pub struct FilterConfig {
    pub skip_split: bool,           // skip splitting fused /Fd<dir> /Fo<dir> flags
    pub skip_bad: bool,             // skip removing bad flags
    pub skip_swap: bool,            // skip swapping problematic flags
    pub skip_add: bool,             // skip appending extra helpful flags
    pub skip_version_on_empty: bool, // skip auto-adding --version when args is empty
    pub force_response_files: bool, // always use a response file (regardless of length)
    pub args_char_limit: usize,     // override for the argument character limit
}

impl FilterConfig {
    // Reads the WRAPPER_* environment variables into a FilterConfig.
    pub fn from_env() -> Self {
        let skipAll = env::var("WRAPPER_SKIP_ALL_FLAGS").is_ok();
        FilterConfig {
            skip_split: skipAll || env::var("WRAPPER_SKIP_SPLIT_FLAGS").is_ok(),
            skip_bad: skipAll || env::var("WRAPPER_SKIP_BAD_FLAGS").is_ok(),
            skip_swap: skipAll || env::var("WRAPPER_SKIP_SWAP_FLAGS").is_ok(),
            skip_add: skipAll || env::var("WRAPPER_SKIP_ADD_FLAGS").is_ok(),
            skip_version_on_empty: env::var("WRAPPER_SKIP_VERSION_ON_EMPTY").is_ok(),
            force_response_files: env::var("WRAPPER_FORCE_RESPONSE_FILES").is_ok(),
            args_char_limit: FilterConfig::argsCharLimit(),
        }
    }

    // Resolves the effective character limit, honoring WRAPPER_ARGS_CHAR_LIMIT.
    fn argsCharLimit() -> usize {
        match env::var("WRAPPER_ARGS_CHAR_LIMIT") {
            Ok(v) => v.trim().parse::<usize>().unwrap_or(ARGS_CHAR_LIMIT),
            Err(_) => ARGS_CHAR_LIMIT,
        }
    }
}

// Applies the filtering pipeline described by `config`. Pure aside from writing the
// response file when args get too long (or are forced through).
fn applyFilter(
    args: Vec<String>,
    config: &FilterConfig,
    BadFlags: Regex,
    SwapPairs: Vec<(Regex, String)>,
    ExtraFlags: String,
) -> Vec<String> {
    // Split fused flag+directory args (e.g. /Fdsome\target\directory -> /Fd + some\target\directory)
    // BEFORE the swap/bad loop, because one fused arg expands into two and the array grows.
    // An arg that is exactly the flag (already standalone) passes through unchanged.
    let mut expanded: Vec<String> = Vec::new();
    if !config.skip_split {
        for arg in args {
            if let Some(cap) = SplitFlags.captures(&arg) {
                let full = cap.get(0).unwrap().as_str();
                if full.len() < arg.len() {                       // fused flag + directory
                    expanded.push(format!("/F{}", cap.get(1).unwrap().as_str()[1..].to_lowercase()));
                    expanded.push(arg[full.len()..].to_string()); // directory part
                } else {                                          // exactly /Fd or /Fo
                    expanded.push(arg);
                }
            } else {
                expanded.push(arg);
            }
        }
    } else {
        expanded = args;
    }

    // final arguments
    let mut finalArgs: Vec<String> = Vec::new();
    for arg in expanded {
        if !config.skip_bad && BadFlags.is_match(&arg) { continue; } // Drop if bad
        let mut newArg = arg.clone();                             // Swap if match
        if !config.skip_swap {
            for (re, swap) in &*SwapPairs {
                if re.is_match(&arg) {
                    newArg = re.replace(&arg, swap.clone()).trim().to_string();
                    break;
                }
            }
        }
        if !newArg.is_empty() {
            finalArgs.push(newArg);
        }
    }

    if finalArgs.is_empty() && !config.skip_version_on_empty {
        finalArgs.push("--version".into());
    } else if !config.skip_add
        && finalArgs.len() > 1
        && !ExtraFlags.is_empty()
        && finalArgs.iter().any(|a| a == "-x")
    {
        let index = finalArgs.iter().position(|a| a == "-x").unwrap();
        if finalArgs[index + 2] != ExtraFlags.split(" ").next().unwrap() {  // Avoid inserting extra flags twice
            finalArgs.splice(index + 2..index + 2, ExtraFlags.split(" ").map(|s| s.into())); }
    }

    if finalArgs.iter().any(|a| a.starts_with('@')) {
        // rsp already used—pass through
    } else if config.force_response_files || finalArgs.join(" ").len() > config.args_char_limit {
        // too long (or forced): make rsp
        let rsp_path = &ResponseFileName.replace(".rsp", &format!("_{}.rsp", id()).to_owned());
        let mut f = File::create(&rsp_path).unwrap();
        for arg in &finalArgs { writeln!(f, "{}", arg).unwrap(); }  // or space/newline
        finalArgs = vec![format!("@{}", &rsp_path)];
    }
    return finalArgs;
}

// Define the struct (your "class")
pub struct Kind {
    pub exe: String,                  // "name-rs.exe"
    pub src: String,                  // "src/name.rs"
    pub args: Vec<String>,            // (arg1, arg2, arg3, ...)
    pub name: (String, String),       // ("wrapper.exe", "name.exe")
    pub EXE: (String, String),        // ("C:/path/to/wrapper/bin/wrapper.exe", "C:/path/to/llvm/bin/name.exe")
    pub family: (EXEFamily, EXEKind), // (LLVM, Compiler), (LLVM, Linker), (MSVC, Compiler), (MSVC, Linker), (UNKNOWN, UNKNOWN)
    pub finalArgs: Vec<String>,       // (arg1, arg2, arg3, ...)
    pub expect: String,               // "name died"
}

// Impl block = where methods live
impl Kind {
    // Constructor
    pub fn new(src: String, args: Vec<String>) -> Self {
        let exe: String = env::current_exe().expect("name.exe").to_str().unwrap_or("").to_string().replace("\\", "/");
        let src: String = src.replace("\\", "/");
        let args: Vec<String> = args;
        let name: (String, String) = getName(&src, &args);
        let EXE: (String, String) = getEXE(&name);
        let family: (EXEFamily, EXEKind) = getEXEFamily(&name);
        let (regex, swap_pairs, extra_flags) = getFlags(&family);
        let finalArgs: Vec<String> = filterArgs(args.clone(), regex, swap_pairs, extra_flags.to_string());
        let expect: String = name.1.clone() + " died";
        
        Kind {
            exe,
            src,
            args,
            name,
            EXE,
            family,
            finalArgs,
            expect,
        }
    }
    
    pub fn getEXE(&self) -> String {
        let (wrapper, exe) = &self.EXE;
        if wrapper != "unknown" {
            return wrapper.clone();
        } else if exe != "unknown" {
            return exe.clone();
        }
        return "unknown".to_string();
    }

    pub fn printInfo(&self) {
        trace!("Received Exe: {}", self.exe);
        trace!("Src File : {}", self.src);
        trace!("Input Args: {:?}", self.args);
        trace!("Processed Name: {:?}", self.name);
        info!("Found EXE: {:?}", self.EXE);
        info!("Family: {:?}", self.family);
        debug!("FinalArgs: {:?}", self.finalArgs);
        trace!("Expect: {}", self.expect);
    }
}

////////////////////////////////////////////////////////////////////////////////////////
//                                  Print Usage Help                                  //
////////////////////////////////////////////////////////////////////////////////////////
/// ANSI SGR (Select Graphic Rendition) codes used to decorate the usage box.
/// Using raw ANSI keeps the wrapper dependency-free while still giving a
/// colourful heading, sections and borders on modern terminals.
mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const CYAN: &str = "\x1b[36m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const MAGENTA: &str = "\x1b[35m";
}

/// Wrap `text` with an SGR code and reset it afterwards.
fn paint(text: &str, code: &str) -> String {
    format!("{code}{text}{}", ansi::RESET)
}

/// Visible width of a string, ignoring ANSI escape sequences (which occupy
/// zero display columns).
fn visible_len(s: &str) -> usize {
    let mut count = 0;
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            for c2 in it.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            count += 1;
        }
    }
    count
}

/// Wrap plain (uncoloured) text to `width` columns at word boundaries.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let sep = if cur.is_empty() { 0 } else { 1 };
        if cur.chars().count() + sep + word.chars().count() > width {
            out.push(cur.clone());
            cur.clear();
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Centre `s` within `inner` columns using visible width.
fn center_text(s: &str, inner: usize) -> String {
    let pad = inner.saturating_sub(visible_len(s));
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

/// A section divider `╠═ title ═══...══╣` with the title coloured.
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
pub fn printUsage() -> bool {
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
        "WRAPPER_SKIP_BAD_FLAGS",
        "Skip removing bad flags.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_SWAP_FLAGS",
        "Skip swapping problematic flags.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_ADD_FLAGS",
        "Skip adding extra helpful flags.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_SPLIT_FLAGS",
        "Skip splitting fused flags.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_ALL_FLAGS",
        "Skip removing bad flags, swapping problematic flags, adding extra helpful flags, and splitting fused flags.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_SKIP_VERSION_ON_EMPTY",
        "Skip automatically adding --version when no arguments are provided.",
        NAME_W,
        INNER,
    ));

    // ---- Response files ----
    lines.push(section_row("RESPONSE FILES", INNER));
    lines.extend(usage_pair(
        "WRAPPER_ARGS_CHAR_LIMIT",
        &format!(
            "Override the default argument character limit of {} to enable response files.",
            ARGS_CHAR_LIMIT
        ),
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_FORCE_RESPONSE_FILES",
        &format!(
            "Force response files regardless of the total argument character length of {}.",
            ARGS_CHAR_LIMIT
        ),
        NAME_W,
        INNER,
    ));
// ---- Logging & debug (RUST_LOG) ----
    lines.push(section_row("LOGGING & DEBUG (RUST_LOG)", INNER));
    lines.extend(usage_pair(
        "RUST_LOG",
        "Set the diagnostic verbosity used by the wrapper. Standard env_logger variable; unset defaults to error.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "error",
        "Log only errors. Least verbose (the default when RUST_LOG is unset).",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "warn",
        "Log warnings and errors.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "info",
        "Log informational messages, warnings and errors.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "debug",
        "Log debug messages plus everything above.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "trace",
        "Log trace messages plus everything above. Most verbose.",
        NAME_W,
        INNER,
    ));

    // ---- Other / help ----
    lines.push(section_row("OTHER & HELP", INNER));
    lines.extend(usage_pair(
        "WRAPPER_ENABLE_PASSTHROUGH",
        "Pass arguments through directly without processing.",
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_OPTIONS or WRAPPER_HELP",
        "Print this help message and exit.",
        NAME_W,
        INNER,
    ));

    // ---- Footer ----
    lines.push(box_row("", INNER));
    lines.push(box_row(
        &center_text(
            &paint("Set any of these environment variables to enable its behaviour.", ansi::DIM),
            INNER,
        ),
        INNER,
    ));
    lines.push(box_row("", INNER));

    // ---- Assemble the box ----
    let mut out = String::new();
    out.push_str(&format!("╔{rule}╗\n"));
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&format!("╚{rule}╝"));

    println!();
    println!("{out}");
    println!();

    true
}

//////////////////////////////////////////////////////////////////////////////////////////
//                                    Unit Tests                                        //
//////////////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    // Serializes response-file tests: the response file name embeds the process
    // id (same for every test thread), so parallel tests would clobber each
    // other's files. Holding this mutex ensures create→read→delete is atomic.
    static RSP_MUTEX: Mutex<()> = Mutex::new(());

    fn never_match() -> Regex {
        regex::Regex::new(r"\P{any}").unwrap() // matches no character; never matches anything
    }

    // Split fused flag+directory: /Fd<path> and /Fo<path> become two args.
    #[test]
    fn splits_fused_flag_and_directory() {
        let args = vec![
            "/Fdsome\\target\\directory".to_string(),
            "/Foanother\\target\\directory".to_string(),
            "main.cpp".to_string(),
        ];
        let result = filterArgs(args, never_match(), vec![], String::new());
        assert_eq!(
            result,
            vec![
                "/Fd".to_string(),
                "some\\target\\directory".to_string(),
                "/Fo".to_string(),
                "another\\target\\directory".to_string(),
                "main.cpp".to_string(),
            ]
        );
    }

    // Standalone flags pass through unchanged.
    #[test]
    fn passes_through_standalone_flags() {
        let args = vec![
            "/Fd".to_string(),
            "/Fo".to_string(),
            "main.cpp".to_string(),
        ];
        let result = filterArgs(args, never_match(), vec![], String::new());
        assert_eq!(
            result,
            vec![
                "/Fd".to_string(),
                "/Fo".to_string(),
                "main.cpp".to_string(),
            ]
        );
    }

    // Case-insensitive flag prefix still splits; case-mismatch standalone passes through.
    #[test]
    fn splits_case_insensitive_prefix() {
        let fused = vec!["/Fodist\\lib.obj".to_string()];
        assert_eq!(
            filterArgs(fused, never_match(), vec![], String::new()),
            vec!["/Fo".to_string(), "dist\\lib.obj".to_string()]
        );
    }

    // Mixed real-world input: -clang: prefixed args pass through unchanged (they start with
    // '-', not '/'), only the leading /Fd fused flag+path splits.
    #[test]
    fn passes_clang_prefixed_and_splits_fd() {
        let args = vec![
            "-clang:-MSmy\\target\\dir.name\\myname\\".to_string(),
            "-clang:/FoCMakeLists\\my\\sussy.dir\\".to_string(),
            "/Fdsome\\suspicious\\dirname".to_string(),
        ];
        let result = filterArgs(args, never_match(), vec![], String::new());
        assert_eq!(
            result,
            vec![
                "-clang:-MSmy\\target\\dir.name\\myname\\".to_string(),
                "-clang:/FoCMakeLists\\my\\sussy.dir\\".to_string(),
                "/Fd".to_string(),
                "some\\suspicious\\dirname".to_string(),
            ]
        );
    }

    // Helper: a config that mimics the default (no skips) but with a large char limit so
    // args never spill into a response file unless explicitly tested.
    fn default_cfg() -> FilterConfig {
        FilterConfig {
            args_char_limit: 1_000_000,
            ..FilterConfig::default()
        }
    }

    // skip_split leaves fused /Fd<dir> untouched instead of splitting into two args.
    #[test]
    fn skip_split_keeps_fused_flag_together() {
        let args = vec!["/Fdsome\\target\\directory".to_string()];
        let mut cfg = default_cfg();
        cfg.skip_split = true;
        let result = applyFilter(args, &cfg, never_match(), vec![], String::new());
        assert_eq!(result, vec!["/Fdsome\\target\\directory".to_string()]);
    }

    // Without skip_split, the fused flag still splits (sanity guard against regressions).
    #[test]
    fn default_splits_fused_flag() {
        let args = vec!["/Fdsome\\target\\directory".to_string()];
        let result = applyFilter(args, &default_cfg(), never_match(), vec![], String::new());
        assert_eq!(
            result,
            vec!["/Fd".to_string(), "some\\target\\directory".to_string()]
        );
    }

    // skip_bad keeps a flagged arg that would otherwise be dropped.
    #[test]
    fn skip_bad_keeps_flags_that_would_be_removed() {
        let args = vec!["/bigobj".to_string(), "main.cpp".to_string()];
        let bad = regex::Regex::new(r"^/bigobj$").unwrap();

        let mut cfg = default_cfg();
        cfg.skip_bad = true;
        assert_eq!(
            applyFilter(args.clone(), &cfg, bad.clone(), vec![], String::new()),
            vec!["/bigobj".to_string(), "main.cpp".to_string()]
        );

        assert_eq!(
            applyFilter(args.clone(), &default_cfg(), bad, vec![], String::new()),
            vec!["main.cpp".to_string()]
        );
    }

    // skip_swap keeps the original arg instead of replacing it.
    #[test]
    fn skip_swap_keeps_original_arg() {
        let args = vec!["/Zi".to_string()];
        let swaps = vec![(regex::Regex::new(r"^/Zi$").unwrap(), "-g".to_string())];

        let mut cfg = default_cfg();
        cfg.skip_swap = true;
        assert_eq!(
            applyFilter(args.clone(), &cfg, never_match(), swaps.clone(), String::new()),
            vec!["/Zi".to_string()]
        );

        assert_eq!(
            applyFilter(args.clone(), &default_cfg(), never_match(), swaps, String::new()),
            vec!["-g".to_string()]
        );
    }

    // skip_add prevents the extra helpful flags from being spliced in after -x.
    #[test]
    fn skip_add_prevents_extra_flags() {
        let args = vec!["-x".to_string(), "a.c".to_string(), "b".to_string()];
        let extra = "FLAG1 FLAG2".to_string();

        let mut cfg = default_cfg();
        cfg.skip_add = true;
        assert_eq!(
            applyFilter(args.clone(), &cfg, never_match(), vec![], extra.clone()),
            vec!["-x".to_string(), "a.c".to_string(), "b".to_string()]
        );

        // Without skip_add the extra flags are spliced in.
        assert_eq!(
            applyFilter(args, &default_cfg(), never_match(), vec![], extra),
            vec![
                "-x".to_string(),
                "a.c".to_string(),
                "FLAG1".to_string(),
                "FLAG2".to_string(),
                "b".to_string()
            ]
        );
    }

    // With no arguments the wrapper auto-appends --version so the tool still prints
    // something useful; WRAPPER_SKIP_VERSION_ON_EMPTY leaves the args list empty.
    #[test]
    fn skip_version_on_empty_prevents_auto_version() {
        let empty: Vec<String> = vec![];

        // Default behaviour: empty args become a lone --version.
        assert_eq!(
            applyFilter(empty.clone(), &default_cfg(), never_match(), vec![], String::new()),
            vec!["--version".to_string()]
        );

        // With the skip set, nothing is appended.
        let mut cfg = default_cfg();
        cfg.skip_version_on_empty = true;
        assert_eq!(
            applyFilter(empty, &cfg, never_match(), vec![], String::new()),
            Vec::<String>::new()
        );
    }

    // skip-all short-circuits: every hop disabled.
    #[test]
    fn skip_all_flags_disables_every_step() {
        let args = vec!["/Fdsome\\dir".to_string(), "/bigobj".to_string()];
        let bad = regex::Regex::new(r"^/bigobj$").unwrap();
        let mut cfg = default_cfg();
        cfg.skip_split = true;
        cfg.skip_bad = true;
        cfg.skip_swap = true;
        cfg.skip_add = true;
        let result = applyFilter(args.clone(), &cfg, bad, vec![], "FLAG".to_string());
        assert_eq!(result, args);
    }

    // force_response_files collapses the final args into a single @file.rsp arg.
    #[test]
    fn force_response_files_uses_response_file() {
        let _guard = RSP_MUTEX.lock().unwrap();
        let args = vec!["main.cpp".to_string()];
        let mut cfg = default_cfg();
        cfg.force_response_files = true;
        let result = applyFilter(args, &cfg, never_match(), vec![], String::new());
        assert_eq!(result.len(), 1);
        let rsp = result[0].strip_prefix('@').unwrap().to_string();
        assert!(rsp.ends_with(".rsp"));
        // Response file must be an absolute path in the temp directory
        let path = std::path::Path::new(&rsp);
        assert!(path.is_absolute(), "response file path should be absolute, got: {}", rsp);
        assert!(rsp.contains("linker_reponse_"), "response file name should contain prefix, got: {}", rsp);
        let _ = std::fs::remove_file(&rsp);
    }

    // When args exceed args_char_limit the wrapper emits a response file.
    #[test]
    fn args_over_char_limit_become_response_file() {
        let _guard = RSP_MUTEX.lock().unwrap();
        let args = vec!["main.cpp".to_string(), "with-a-very-long-name.cpp".to_string()];
        let mut cfg = default_cfg();
        cfg.args_char_limit = 10; // shorter than the joined args
        let result = applyFilter(args, &cfg, never_match(), vec![], String::new());
        assert_eq!(result.len(), 1);
        let rsp = result[0].strip_prefix('@').unwrap().to_string();
        assert!(rsp.ends_with(".rsp"));
        let path = std::path::Path::new(&rsp);
        assert!(path.is_absolute(), "response file path should be absolute, got: {}", rsp);
        let _ = std::fs::remove_file(&rsp);
    }

    // Verifies the response file round-trip: args written to the .rsp file are
    // exactly the input args, one per line, in order.
    #[test]
    fn response_file_content_matches_args() {
        let _guard = RSP_MUTEX.lock().unwrap();
        let args = vec![
            "-O2".to_string(),
            "-I/usr/local/include".to_string(),
            "main.cpp".to_string(),
            "-o".to_string(),
            "output.exe".to_string(),
        ];
        let mut cfg = default_cfg();
        cfg.force_response_files = true;
        let result = applyFilter(args.clone(), &cfg, never_match(), vec![], String::new());
        assert_eq!(result.len(), 1);

        let rsp_path = result[0].strip_prefix('@').unwrap().to_string();
        assert!(rsp_path.ends_with(".rsp"));
        assert!(std::path::Path::new(&rsp_path).is_absolute());

        // Read the response file back and verify each line matches the original args
        let content = std::fs::read_to_string(&rsp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), args.len());
        for (line, expected) in lines.iter().zip(args.iter()) {
            assert_eq!(line, expected);
        }

        let _ = std::fs::remove_file(&rsp_path);
    }

    // Verifies that args with special characters (spaces, dashes, paths) survive the round-trip.
    #[test]
    fn response_file_handles_special_arg_characters() {
        let _guard = RSP_MUTEX.lock().unwrap();
        let args = vec![
            "-DFOO=bar".to_string(),
            "-isystemC:\\Program Files\\Include".to_string(),
            "/Fd".to_string(),
            "C:\\My Projects\\debug\\".to_string(),
            "-std=c++20".to_string(),
        ];
        let mut cfg = default_cfg();
        cfg.force_response_files = true;
        let result = applyFilter(args.clone(), &cfg, never_match(), vec![], String::new());
        assert_eq!(result.len(), 1);

        let rsp_path = result[0].strip_prefix('@').unwrap().to_string();
        assert!(std::path::Path::new(&rsp_path).is_absolute());
        let content = std::fs::read_to_string(&rsp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), args.len());
        for (line, expected) in lines.iter().zip(args.iter()) {
            assert_eq!(line, expected);
        }

        let _ = std::fs::remove_file(&rsp_path);
    }
}
