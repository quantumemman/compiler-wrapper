use std::env;                             // for environment variables
use std::fs::File;                        // for File operations
use std::io::Write;                       // for Write trait
use std::path::{Path, PathBuf};           // for Path operations
use std::process::{id};                   // for process id
use std::string::String;                  // for String type
use std::sync::LazyLock;                  // for LazyLock init
use regex::Regex;                         // for Regex operations
use log::{trace, debug, info, warn};      // logging macros with levels: trace, debug, info, warn, error

/////////////////////////////////////////////////////////////////////////////////////////
//                         Define Executables Search Paths                             //
/////////////////////////////////////////////////////////////////////////////////////////
pub const NVCC_PATH: &str = env!("NVCC_PATH_");           // NVIDIA CUDA compiler path
pub const LLVM_PATH_VS: &str = env!("LLVM_PATH_VS_");     // Visual Studio LLVM toolchain path
pub const MSVC_PATH: &str = env!("MSVC_PATH_");           // Microsoft Visual C++ toolchain path
pub const LLVM_PATH: &str = env!("LLVM_PATH_");           // NVIDIA/AMD LLVM toolchain path
pub const GCC_PATH: &str = env!("GCC_PATH_");             // GNU Compiler Collection path
pub const PY_SCRIPTS_PATH: &str = env!("PY_PATH_");       // Python venv Scripts path
pub static PATHS: LazyLock<[&str; 6]> = LazyLock::new(|| {
    // If WRAPPER_PREFER_VS is defined, prefer Visual Studio's LLVM toolchain; otherwise prefer NVIDIA/AMD LLVM.
    if env::var("WRAPPER_PREFER_VS").is_ok() {
        info!("Preferring Visual Studio LLVM toolchain");
        [&NVCC_PATH, &LLVM_PATH_VS, &MSVC_PATH, &LLVM_PATH, &GCC_PATH, &PY_SCRIPTS_PATH] // Prefer VS LLVM
    } else {
        info!("Preferring NVIDIA/AMD LLVM toolchain");
        [&NVCC_PATH, &LLVM_PATH, &LLVM_PATH_VS, &MSVC_PATH, &GCC_PATH, &PY_SCRIPTS_PATH] // Prefer NVIDIA/AMD LLVM
    }
});

/////////////////////////////////////////////////////////////////////////////////////////
//                            Define Executables Keywords                              //
/////////////////////////////////////////////////////////////////////////////////////////
pub static WRAPPER_KEYWORDS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)ccache"#).unwrap());
pub static EXTERNAL_WRAPPER_SIGNATURE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)[-](rs)"#).unwrap()); // matches executable names that are external wrappers like clang-rs to avoid doing their work for them since they will be called by this program
pub static COMPILER_KEYWORDS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(clang|hip|cl|gcc|g\+\+)"#).unwrap());
pub static LINKER_KEYWORDS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(link|lld|ld)"#).unwrap());

pub static LLVM_KEYWORDS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(clang|llvm|lld)"#).unwrap());
pub static MSVC_KEYWORDS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(cl|link)"#).unwrap());
pub static GCC_KEYWORDS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"(?i)(gcc|g\+\+|ld)"#).unwrap());

/////////////////////////////////////////////////////////////////////////////////////////
//                                Define Bad Flag Regexes                              //
/////////////////////////////////////////////////////////////////////////////////////////
pub static LLVM_COMPILER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"^[-/](permissive-|bigobj|EGR|W3|Wc\+\+11-narrowing|Wincompatible-pointer-types|Wimplicit-function-declaration|Wdeprecated-declarations|Wextern-initializer|Wold-style-cast|Wunused-variable|Wunused-function|Wunused-command-line-argument|Wlogical-op-parentheses|Wignored-attributes|Wunknown-warning-option)$"#).unwrap());
pub static MSVC_COMPILER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"^[-/](bigobj|GR|Od|W3|Wc\+\+11-narrowing|Wincompatible-pointer-types|Wimplicit-function-declaration|Wdeprecated-declarations|Wextern-initializer|Wold-style-cast|Wunused-variable|Wunused-function|Wunused-command-line-argument|Wlogical-op-parentheses|Wignored-attributes|Wunknown-warning-option)$"#).unwrap());
pub static GCC_COMPILER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(||Regex::new(r#"^[-/](Werror|ffast-math|fstrict-aliasing|fpack-struct|fshort-enum)"#).unwrap());

pub static LLVM_LINKER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[/]INCREMENTAL:NO$"#).unwrap());
pub static MSVC_LINKER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[/]INCREMENTAL:NO$"#).unwrap());
pub static GCC_LINKER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[-/](Werror)"#).unwrap());

pub static COMMON_SPLIT_FLAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[-/](Fd|Fo)"#).unwrap());

/////////////////////////////////////////////////////////////////////////////////////////
//                             Swap Problematic Arguments                              //
/////////////////////////////////////////////////////////////////////////////////////////
pub static LLVM_COMPILER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[-]D[/]bigobj$").expect(BAD_MATCH_MESSAGE), "".into()),
        (Regex::new(r"^[/]MD\(d\)?$").expect(BAD_MATCH_MESSAGE), "-fms-extensions".into()),
        (Regex::new(r"^[/]Zi$").expect(BAD_MATCH_MESSAGE), "-g".into()),
        (Regex::new(r"^[/]O1$").expect(BAD_MATCH_MESSAGE), "-O1".into()),
        (Regex::new(r"^[/]O2$").expect(BAD_MATCH_MESSAGE), "-O2".into()),
        (Regex::new(r"^[/]O3$").expect(BAD_MATCH_MESSAGE), "-O3".into()),
        (Regex::new(r"^[/]O4$").expect(BAD_MATCH_MESSAGE), "-O4".into()),
]});

pub static LLVM_LINKER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BAD_MATCH_MESSAGE), "-flto".into()),
        (Regex::new(r"^[/]?MANIFEST:(EMBED(,ID=\d+)?)$").expect(BAD_MATCH_MESSAGE), "/MANIFEST:NO".into()),
]});

pub static MSVC_COMPILER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]Zi$").expect(BAD_MATCH_MESSAGE), "/Z7".into()),
]});

pub static MSVC_LINKER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BAD_MATCH_MESSAGE), "-flto".into()),
        (Regex::new(r"^[/]?MANIFEST:(EMBED(,ID=\d+)?)$").expect(BAD_MATCH_MESSAGE), "/MANIFEST:NO".into()),
]});

pub static GCC_COMPILER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]Zi$").expect(BAD_MATCH_MESSAGE), "-g".into()),
]});

pub static GCC_LINKER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(||{
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BAD_MATCH_MESSAGE), "-flto".into()),
]});

/////////////////////////////////////////////////////////////////////////////////////////
//                                  Define Extra Flags                                 //
/////////////////////////////////////////////////////////////////////////////////////////
pub const LLVM_COMPILER_EXTRA_FLAGS: &str = "-D_USE_MATH_DEFINES -D_CRT_SECURE_NO_WARNINGS -w -Wno-deprecated -Wno-author -Wno-unused-cli -Wno-ignored-attributes";
pub const LLVM_LINKER_EXTRA_FLAGS: &str = "/MANIFEST:NO";
pub const MSVC_COMPILER_EXTRA_FLAGS: &str = "-D_USE_MATH_DEFINES -D_CRT_SECURE_NO_WARNINGS -FS -w";
pub const MSVC_LINKER_EXTRA_FLAGS: &str = "";
pub const GCC_COMPILER_EXTRA_FLAGS: &str = "-w";
pub const GCC_LINKER_EXTRA_FLAGS: &str = "";

pub const CLI_ARGS_CHAR_LIMIT: usize = 30000;  // maximum CLI args char length before using response file
pub const UNKNOWN_KEYWORD: &str = "UNKNOWN";
pub const BAD_MATCH_MESSAGE: &str = "bad match";
pub const RESPONSE_FILE_NAME: &str = "response_file.rsp";

/////////////////////////////////////////////////////////////////////////////////////////
//                       Define Compiler/Linker Classification Enums                   //
/////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub enum ExecutableFamily {
    LLVM,        // clang, clang++, clang-cl, lld-link
    MSVC,        // cl, link, can put clang-cl here via env vars
    GCC,         // gcc, g++, ld
    UNKNOWN,     // default classification, sccache, ccache
}

#[derive(Debug)]
pub enum ExecutableKind {
    COMPILER,    // clang, clang-cl, cl, gcc, g++, hipcc
    LINKER,      // link, lld-link, ld
    WRAPPER,     // sccache, ccache, used to skip arg filtering since the exe type is unknown but not problematic
    UNKNOWN,     // default classification, this will correspond to a new exe type
}

/////////////////////////////////////////////////////////////////////////////////////////
//                                Get Executable Names                                 //
/////////////////////////////////////////////////////////////////////////////////////////
pub fn get_executable_names(src_executable: &String, input_args: &mut Vec<String>) -> (String, String) {
    let mut wrapper_name = UNKNOWN_KEYWORD.to_string();
    let mut executable_name = wrapper_name.clone();
    
    if WRAPPER_KEYWORDS.is_match(src_executable) {                                                   // matches a wrapper keyword such as sccache
        if COMPILER_KEYWORDS.is_match(src_executable) || LINKER_KEYWORDS.is_match(src_executable) {  // e.g. sccache-clang-cl
            // The combined form is always "<wrapper>-<tool>", e.g. "sccache-clang-cl"
            // or "ccache-g++". Split on the first '-' so the wrapper prefix is
            // stripped generically — no need to know whether it is sccache,
            // ccache, wrapper, or any other driver name.
            if let Some((wrapper, tool)) = src_executable.split_once('-') {
                wrapper_name = wrapper.to_string();
                executable_name = tool.to_string();
            }
        } else {                                        // this is the case of pure sccache being called
                wrapper_name = src_executable.clone();  // keep the wrapper executable as is
                executable_name = input_args.remove(0); // the executable should be the next arg since wrappers are not compilers or linkers
        }
    } else {
        executable_name = src_executable.clone();  // there is no wrapper in this scenario so store it as the executable
    }
    (wrapper_name, executable_name)
}

//////////////////////////////////////////////////////////////////////////////////////////
//                                   Locate Executables                                 //
//////////////////////////////////////////////////////////////////////////////////////////
fn find_executable(executable_name: &str, paths: &[&str]) -> Option<PathBuf> {
    if executable_name != UNKNOWN_KEYWORD {
        if !(EXTERNAL_WRAPPER_SIGNATURE.is_match(&executable_name) || Path::new(executable_name).is_absolute()) {
            for dir in paths {
                // trace!("Searching for {} in {}", executable_name, dir);
                let candidate = Path::new(dir).join(executable_name);
                // On Windows, executables have the .exe extension; on other platforms they typically don't.
                let candidate = if cfg!(windows) {
                    candidate.with_extension("exe")
                } else {
                    candidate
                };
                // trace!("Checking candidate path: {:?}", candidate);
                if candidate.exists() && candidate.is_file() {
                    trace!("Found {} in {}", executable_name, dir);
                    return Some(candidate.to_path_buf());
                }
            }
        } else {
            trace!("Executable is absolute or another wrapper, returning as is: {}", executable_name);
            return Some(PathBuf::from(executable_name));
        }
    }
    None
}

/////////////////////////////////////////////////////////////////////////////////////////
//                                  Get Executable Paths                               //
/////////////////////////////////////////////////////////////////////////////////////////
pub fn get_executable_paths(target_executable_names: &(String, String)) -> (String, String) {
    let wrapper_path: String = find_executable(&target_executable_names.0, &*PATHS).unwrap_or(UNKNOWN_KEYWORD.into()).to_str().unwrap_or("").to_string();    
    let executable_path: String = find_executable(&target_executable_names.1, &*PATHS).unwrap_or(UNKNOWN_KEYWORD.into()).to_str().unwrap_or("").to_string();
    return (wrapper_path.replace("\\", "/"), executable_path.replace("\\", "/"));
}

/////////////////////////////////////////////////////////////////////////////////////////
//                      Determine Main and Deputy Executable Paths                     //
/////////////////////////////////////////////////////////////////////////////////////////
pub fn get_main_and_deputy_executable_paths(target_executable_names: &(String, String)) -> (String, String) {
    let deputy_executable: String = target_executable_names.1.clone();
    let main_executable : String = if target_executable_names.0 != UNKNOWN_KEYWORD { target_executable_names.0.clone() } else { deputy_executable.clone() };
    (main_executable, deputy_executable)
}

///////////////////////////////////////////////////////////////////////////////////////////////
//                               Classify Executable Types                                   //
///////////////////////////////////////////////////////////////////////////////////////////////
// Classify the exe kind based on its name matching keywords
pub fn get_target_classification(executable_name: &String) -> (ExecutableFamily, ExecutableKind) {
    let mut executable_family = ExecutableFamily::UNKNOWN;
    let mut executable_kind = ExecutableKind::UNKNOWN;
    
    // treat clang-cl as MSVC only when explicitly requested AND LLVM is not requested
    if (env::var("WRAPPER_CLANG_CL_IS_MSVC").is_ok() && !env::var("WRAPPER_CLANG_CL_IS_LLVM").is_ok()) && executable_name.contains("clang-cl") {
        executable_family = ExecutableFamily::MSVC;
    } else if LLVM_KEYWORDS.is_match(executable_name) {     // LLVM family
        executable_family = ExecutableFamily::LLVM;
    } else if MSVC_KEYWORDS.is_match(executable_name) {     // MSVC family
        executable_family = ExecutableFamily::MSVC;
    } else if GCC_KEYWORDS.is_match(executable_name) {      // GCC family
        executable_family = ExecutableFamily::GCC;
    }
    
    if COMPILER_KEYWORDS.is_match(executable_name) {        // prioritize compilers since it is the most in need of this wrapper
        executable_kind = ExecutableKind::COMPILER;
    } else if LINKER_KEYWORDS.is_match(executable_name) {   // next do linker
        executable_kind = ExecutableKind::LINKER;
    }
    
    (executable_family, executable_kind)
}

/////////////////////////////////////////////////////////////////////////////////////////
//                            Get the Arguments Filter Pack                            //
/////////////////////////////////////////////////////////////////////////////////////////
pub fn get_args_filter_pack(family: &(ExecutableFamily, ExecutableKind)) -> (Regex, Vec<(Regex, String)>, &str) {
    match family {
        (ExecutableFamily::LLVM, ExecutableKind::COMPILER) => (LLVM_COMPILER_BAD_FLAGS.clone(), LLVM_COMPILER_SWAP_PAIRS.clone(), LLVM_COMPILER_EXTRA_FLAGS),
        (ExecutableFamily::MSVC, ExecutableKind::COMPILER) => (MSVC_COMPILER_BAD_FLAGS.clone(), MSVC_COMPILER_SWAP_PAIRS.clone(), MSVC_COMPILER_EXTRA_FLAGS),
        (ExecutableFamily::GCC, ExecutableKind::COMPILER) => (GCC_COMPILER_BAD_FLAGS.clone(), GCC_COMPILER_SWAP_PAIRS.clone(), GCC_COMPILER_EXTRA_FLAGS),
        (ExecutableFamily::LLVM, ExecutableKind::LINKER) => (LLVM_LINKER_BAD_FLAGS.clone(), LLVM_LINKER_SWAP_PAIRS.clone(), LLVM_LINKER_EXTRA_FLAGS),
        (ExecutableFamily::MSVC, ExecutableKind::LINKER) => (MSVC_LINKER_BAD_FLAGS.clone(), MSVC_LINKER_SWAP_PAIRS.clone(), MSVC_LINKER_EXTRA_FLAGS),
        (ExecutableFamily::GCC, ExecutableKind::LINKER) => (GCC_LINKER_BAD_FLAGS.clone(), GCC_LINKER_SWAP_PAIRS.clone(), GCC_LINKER_EXTRA_FLAGS),
        _ => panic!("Invalid executable family and kind"), // Add new cases when triggered
    }
}

/////////////////////////////////////////////////////////////////////////////////////////
//                                   Filter Arguments                                  //
/////////////////////////////////////////////////////////////////////////////////////////
pub fn filter_args(input_args: Vec<String>, bad_flags: &Regex, swap_pairs: &Vec<(Regex, String)>, extra_flags: &String) -> Vec<String> {
    // Pass through everything untouched, no processing whatsoever.
    if env::var("WRAPPER_ENABLE_PASSTHROUGH").is_ok() {
        return input_args;
    }
    let config = FilterConfig::from_env();
    apply_filter(input_args, &config, &bad_flags, &swap_pairs, &extra_flags)
}

// Applies the filtering pipeline described by `config`. Pure aside from writing the response file when args get too long (or are forced through).
fn apply_filter(input_args: Vec<String>, config: &FilterConfig, bad_flags: &Regex, swap_pairs: &Vec<(Regex, String)>, extra_flags: &String) -> Vec<String> {
    // Split fused flag+directory args (e.g. /Fdsome\target\directory -> /Fd + some\target\directory)
    // BEFORE the swap/bad loop, because one fused arg expands into two and the array grows.
    // An arg that is exactly the flag (already standalone) passes through unchanged.
    let mut expanded_args: Vec<String> = Vec::new();
    if !config.skip_split {
        for arg in input_args {
            if let Some(cap) = COMMON_SPLIT_FLAGS.captures(&arg) {
                // The matched text is the whole fused flag, e.g. "/Fo" or "-Fd". It is
                // exactly the split-out flag (prefix preserved), so reuse it directly.
                let flag = cap.get(0).unwrap().as_str();
                if flag.len() < arg.len() {                          // fused flag + directory
                    expanded_args.push(flag.to_string());
                    expanded_args.push(arg[flag.len()..].to_string()); // directory part
                } else {                                             // bare flag, no value
                    expanded_args.push(arg);
                }
            } else {
                expanded_args.push(arg);
            }
        }
    } else {
        expanded_args = input_args;
    }

    // final arguments
    let mut final_args: Vec<String> = Vec::new();
    for arg in expanded_args {
        if !config.skip_bad && bad_flags.is_match(&arg) { continue; } // Drop if bad
        let mut new_arg = arg.clone();                             // Swap if match
        if !config.skip_swap {
            for (re, swap) in &*swap_pairs {
                if re.is_match(&arg) {
                    new_arg = re.replace(&arg, swap.clone()).trim().to_string();
                    break;
                }
            }
        }
        if !new_arg.is_empty() {
            final_args.push(new_arg);
        }
    }

    if final_args.is_empty() && !config.skip_version_on_empty {
        final_args.push("--version".into());
    } else if !config.skip_add && !extra_flags.is_empty() {
        // Splice in the extra helpful flags at a position that cannot corrupt the
        // compile/link command — see `insert_extra_flags` for the placement rules.
        insert_extra_flags(&mut final_args, &split_flags(extra_flags));
    }

    if final_args.iter().any(|a| a.starts_with('@')) {
        // rsp already used — pass through
        info!("Response file already in use. Pass through.");
    } else if config.force_response_files || final_args.join(" ").len() > config.args_char_limit {
        // too long (or forced): make rsp
        // Use an absolute path in the temp directory so the spawned compiler can
        // always find the file regardless of its working directory.
        let rsp_name = RESPONSE_FILE_NAME.replace(".rsp", &format!("_{}.rsp", id()));
        let rsp_path = env::temp_dir().join(&rsp_name);
        warn!("Creating response file: {}.", rsp_path.display());
        let mut f = File::create(&rsp_path).unwrap();
        for arg in &final_args { writeln!(f, "{}", arg).unwrap(); }
        final_args = vec![format!("@{}", rsp_path.display())];
    }
    return final_args;
}

// Tokenize an `extra_flags` string on ASCII whitespace. Using split_whitespace
// (not split(" ")) avoids leaking empty tokens on leading/trailing/doubled spaces.
fn split_flags(extra_flags: &str) -> Vec<String> {
    extra_flags.split_whitespace().map(|s| s.to_string()).collect()
}

// Extensions that mark an argument as a compilation input. Used to tell a compile
// step apart from a pure link step so compiler-only flags (warning suppressions,
// `-D` defines) are never spliced into a linker invocation.
const SOURCE_EXTENSIONS: &[&str] = &[
    "c", "cc", "cpp", "cxx", "c++", "m", "mm", "cu", "cuh", "s", "asm",
];

// True when `a` names a source file (e.g. "main.cpp", "/x/foo.c"). Flags and
// response-file arguments are never source files.
fn is_source_arg(a: &str) -> bool {
    if a.starts_with('-') || a.starts_with('/') || a.starts_with('@') {
        return false;
    }
    Path::new(a)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|ext| SOURCE_EXTENSIONS.contains(&ext.as_str()))
}

// Heuristic: is this a compile step rather than a link step? True when `-x`/`-c`/
// `-S`/`-E` appear, or a non-flag argument names a source file. Object/library/
// response-file inputs alone imply a link.
fn is_compile_step(args: &[String]) -> bool {
    if args.iter().any(|a| matches!(a.as_str(), "-x" | "-c" | "-S" | "-E")) {
        return true;
    }
    args.iter().any(|a| is_source_arg(a))
}

// Insert `extra` tokens at `at`, preserving the relative order of every other
// argument so no flag/value pair is ever split.
fn splice_at(args: &mut Vec<String>, at: usize, extra: &[String]) {
    let tail = args.split_off(at);
    args.extend(extra.iter().cloned());
    args.extend(tail);
}

// Safely splice the extra helpful flags into `args` so the resulting compile/link
// command stays valid. The flags are compiler options, so they are always placed in
// the *options* region of the command — never appended after a `--` / the source
// file, where clang-cl treats trailing tokens as linker/input arguments.
//
// Placement rules, in priority order:
//   * `-x <lang>` anchor: flags land right after the language token. Bounds-checked
//     so a bare/trailing `-x` can no longer panic (the old code indexed i+2 blindly).
//   * `--` separator: everything after `--` is positional, so flags are inserted just
//     before it (this is the CMake/clang-cl shape: `... /Fo<o> /Fd<d> -c -- src.c`).
//   * otherwise, before the first source file — the natural end of the options list;
//     stopping only at a source token guarantees a preceding `-o <out>` / `-Fo...`
//     flag+value pair is never split.
//   * fallback: only append at the end for a clear compile step; a pure link (objects
//     + `-o`) is left untouched so compiler-only flags never leak into a link.
//   * Duplicate sets and empty/passthrough lists are ignored.
fn insert_extra_flags(args: &mut Vec<String>, extra: &[String]) {
    if args.is_empty() || extra.is_empty() {
        return;
    }
    // If any token is already present, assume the whole set was added before.
    if extra.iter().any(|f| args.contains(f)) {
        return;
    }

    if let Some(x_pos) = args.iter().position(|a| a == "-x") {
        let mut at = x_pos + 1;
        if at < args.len() {
            at += 1; // skip the language token, land before the inputs
        }
        splice_at(args, at, extra);
        return;
    }

    // `--` ends option parsing: everything after it is positional (input/link) args.
    if let Some(sep) = args.iter().position(|a| a == "--") {
        splice_at(args, sep, extra);
        return;
    }

    // No `--`: stop at the start of the source list, keeping options together ahead.
    if let Some(src) = args.iter().position(|a| is_source_arg(a)) {
        splice_at(args, src, extra);
        return;
    }

    // No `-x`/`--`/source: only add flags to a clear compile step; leave links alone.
    if is_compile_step(args) {
        splice_at(args, args.len(), extra);
    }
}

/////////////////////////////////////////////////////////////////////////////////////////
//                   Define the Runtime struct to hold runtime info                    //
/////////////////////////////////////////////////////////////////////////////////////////
pub struct Runtime {
    pub src_file: String,                      // "src/executable-name.rs" e.g. "src/clang-cl.rs"
    pub src_executable: String,                // "executable-name-rs.exe" e.g. "clang-cl-rs.exe"
    pub input_args: Vec<String>,               // (arg1, arg2, arg3, ...) e.g. ("/bigobj", "-c", "file.cpp")
    pub target_executable_names: (String, String), // ("wrapper-name.exe", "executable-name.exe") e.g. ("sccache.exe", "clang-cl.exe")
    pub target_executable_paths: (String, String), // ("/path/to/wrapper-name.exe", "/path/to/executable-name.exe") e.g. ("/path/to/sccache.exe", "/path/to/clang-cl.exe")
    pub target_classification: (ExecutableFamily, ExecutableKind), // (LLVM, Compiler), (LLVM, Linker), (MSVC, Compiler), (MSVC, Linker), (UNKNOWN, UNKNOWN)
    pub main_exe: String,                      // "/path/to/main.exe", left-most valid executable in target_executable_paths e.g. "/path/to/sccache.exe"
    pub deputy_exe: String,                    // "/path/to/deputy.exe", right-most valid executable in target_executable_paths, e.g. "/path/to/clang-cl.exe", may be main.exe or even the first arg in final_args
    pub final_args: Vec<String>,               // (arg1, arg2, arg3, ...) e.g. ("-D/bigobj", "-c", "file.cpp")
    pub expect: String,                        // "executable-name.exe died" e.g. "clang-cl.exe died"
}

/////////////////////////////////////////////////////////////////////////////////////////
//                           Define the Runtime struct methods                         //
/////////////////////////////////////////////////////////////////////////////////////////
// Impl block = where methods live
impl Runtime {
    // Constructor
    pub fn new(src_file: String, mut input_args: Vec<String>) -> Self {
        let mut final_args: Vec<String>;
        let src_executable = Path::new(&src_file).file_name().unwrap().to_str().unwrap().to_string().replace(".rs", "");  // executables end in .exe on Windows but not on other platforms.
        trace!("Src executable: {}", src_executable);
        
        let target_executable_names: (String, String) = get_executable_names(&src_executable, &mut input_args);
        trace!("Target executable names: {:?}", target_executable_names);
        
        let target_executable_paths: (String, String) = get_executable_paths(&target_executable_names);
        trace!("Target executable paths: {:?}", target_executable_paths);
        
        let (main_exe, deputy_exe): (String, String) = get_main_and_deputy_executable_paths(&target_executable_paths);
        trace!("Main exe: {}, Deputy exe: {}", main_exe, deputy_exe);
        
        let target_classification: (ExecutableFamily, ExecutableKind) = get_target_classification(&deputy_exe);
        trace!("Target classification: {:?}", target_classification);
        
        let (bad_flags, swap_pairs, extra_flags) = get_args_filter_pack(&target_classification);
        
        if !EXTERNAL_WRAPPER_SIGNATURE.is_match(&deputy_exe) {
            final_args = filter_args(input_args.clone(), &bad_flags, &swap_pairs, &extra_flags.to_string());
        } else {
            final_args = input_args.clone()
        }
        
        if target_executable_names.0 != UNKNOWN_KEYWORD {final_args.insert(0, deputy_exe.clone())}
        let expect: String = deputy_exe.clone() + " died";

        Runtime {
            src_file: src_file,
            src_executable: src_executable,
            input_args: input_args,
            target_executable_names: target_executable_names,
            target_executable_paths: target_executable_paths,
            target_classification: target_classification,
            main_exe: main_exe,
            deputy_exe: deputy_exe,
            final_args: final_args,
            expect: expect,
        }
    }

    pub fn print_info(&self) {
        trace!("Src File: {}", self.src_file);
        trace!("Src Executable: {}", self.src_executable);
        trace!("Input Args: {:?}", self.input_args);
        trace!("Target Executable Names: {:?}", self.target_executable_names);
        info!("Target Executable Paths: {:?}", self.target_executable_paths);
        info!("Target Classification: {:?}", self.target_classification);
        debug!("Final Args: {:?}", self.final_args);
        trace!("Expect: {}", self.expect);
    }
}

/////////////////////////////////////////////////////////////////////////////////////////
//                Define FilterConfig struct to Process Env Var Options                //
/////////////////////////////////////////////////////////////////////////////////////////
// Per-call configuration controlling which filtering steps run. Derived from env vars
// in `filterArgs`; kept as data so the pure logic in `applyFilter` is unit-testable
// without mutating global process environment (which is unsafe across parallel tests).
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
            Ok(v) => v.trim().parse::<usize>().unwrap_or(CLI_ARGS_CHAR_LIMIT),
            Err(_) => CLI_ARGS_CHAR_LIMIT,
        }
    }
}

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
        "WRAPPER_CLANG_CL_IS_MSVC",
        "Force clang-cl to use the MSVC family instead of the default LLVM family.",
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
        "WRAPPER_SPLIT_FLAGS",
        "Split fused /Fd-dir /Fo-dir flags (off by default).",
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
            CLI_ARGS_CHAR_LIMIT
        ),
        NAME_W,
        INNER,
    ));
    lines.extend(usage_pair(
        "WRAPPER_FORCE_RESPONSE_FILES",
        &format!(
            "Force response files regardless of the total argument character length of {}.",
            CLI_ARGS_CHAR_LIMIT
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
    use regex::Regex;

    use std::sync::Mutex;

    // Serializes response-file tests: the response file name embeds the process
    // id (same for every test thread), so parallel tests would clobber each
    // other's files. Holding this mutex keeps create→read→delete atomic.
    static RSP_MUTEX: Mutex<()> = Mutex::new(());

    // A regex that matches nothing, so no argument is ever dropped or swapped by it.
    fn never_match() -> Regex {
        Regex::new(r"\P{any}").unwrap()
    }

    // Config like the production defaults but with a huge char limit so arguments
    // never spill into a response file unless explicitly tested.
    fn default_cfg() -> FilterConfig {
        FilterConfig {
            args_char_limit: 1_000_000,
            ..FilterConfig::default()
        }
    }

    fn no_swaps() -> Vec<(Regex, String)> {
        vec![]
    }

    fn no_extra() -> String {
        String::new()
    }

    // ---- Executable name resolution -------------------------------------

    #[test]
    fn wrapper_prefix_is_stripped_without_knowing_wrapper_name() {
        // The outer gate recognises any wrapper name containing "ccache". The
        // strip must not hardcode sccache/ccache: it takes whatever precedes the
        // first '-' as the driver, so any such prefix yields the right pair.
        let mut args = vec!["main.cpp".to_string()];
        let (wrapper, tool) = get_executable_names(&"myccache-gcc".to_string(), &mut args);
        assert_eq!(wrapper, "myccache");
        assert_eq!(tool, "gcc");
        assert_eq!(args, vec!["main.cpp"]); // combined wrappers do not consume args
    }

    #[test]
    fn sccache_prefixed_names_split_into_driver_and_tool() {
        for (full, expect_wrapper, expect_tool) in [
            ("sccache-clang", "sccache", "clang"),
            ("sccache-clang-cl", "sccache", "clang-cl"),
            ("sccache-g++", "sccache", "g++"),
            ("sccache-hipcc", "sccache", "hipcc"),
        ] {
            let mut args = Vec::new();
            let (wrapper, tool) = get_executable_names(&full.to_string(), &mut args);
            assert_eq!(wrapper, expect_wrapper, "wrapper for {full}");
            assert_eq!(tool, expect_tool, "tool for {full}");
        }
    }

    #[test]
    fn ccache_prefixed_names_split_into_driver_and_tool() {
        let mut args = Vec::new();
        let (wrapper, tool) = get_executable_names(&"ccache-gcc".to_string(), &mut args);
        assert_eq!(wrapper, "ccache");
        assert_eq!(tool, "gcc");
    }

    #[test]
    fn bare_wrapper_consumes_first_arg_as_tool() {
        let mut args = vec!["gcc".to_string(), "-c".to_string(), "a.c".to_string()];
        let (wrapper, tool) = get_executable_names(&"sccache".to_string(), &mut args);
        assert_eq!(wrapper, "sccache");
        assert_eq!(tool, "gcc");
        assert_eq!(args, vec!["-c".to_string(), "a.c".to_string()]);
    }

    #[test]
    fn non_wrapper_name_passes_through_unchanged() {
        // Names without a wrapper keyword have no driver; the name is the tool.
        let mut args = vec!["main.cpp".to_string()];
        let (wrapper, tool) = get_executable_names(&"clang-cl".to_string(), &mut args);
        assert_eq!(wrapper, "UNKNOWN");
        assert_eq!(tool, "clang-cl");
        assert_eq!(args, vec!["main.cpp"]);
    }

    // ---- Split step -------------------------------------------------------

    #[test]
    fn splits_fused_flag_and_directory() {
        let input_args = vec![
            "/Fdsome\\target\\directory".to_string(),
            "/Foanother\\target\\directory".to_string(),
            "main.cpp".to_string(),
        ];
        let result = apply_filter(input_args, &default_cfg(), &never_match(), &no_swaps(), &no_extra());
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

    #[test]
    fn passes_through_standalone_flags() {
        let input_args = vec![
            "/Fd".to_string(),
            "/Fo".to_string(),
            "main.cpp".to_string(),
        ];
        let result = apply_filter(input_args, &default_cfg(), &never_match(), &no_swaps(), &no_extra());
        assert_eq!(
            result,
            vec![
                "/Fd".to_string(),
                "/Fo".to_string(),
                "main.cpp".to_string(),
            ]
        );
    }

    #[test]
    fn splits_case_insensitive_prefix() {
        assert_eq!(
            apply_filter(
                vec!["/Fodist\\lib.obj".to_string()],
                &default_cfg(),
                &never_match(),
                &no_swaps(),
                &no_extra(),
            ),
            vec!["/Fo".to_string(), "dist\\lib.obj".to_string()]
        );
    }

    #[test]
    fn passes_clang_prefixed_and_splits_fd() {
        let input_args = vec![
            "-clang:-MSmy\\target\\dir.name\\myname\\".to_string(),
            "-clang:/FoCMakeLists\\my\\sussy.dir\\".to_string(),
            "/Fdsome\\suspicious\\dirname".to_string(),
        ];
        let result = apply_filter(input_args, &default_cfg(), &never_match(), &no_swaps(), &no_extra());
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

    // The split step now accepts both "/" and "-" prefixes for the fused
    // Fd/Fo flags, preserving whichever prefix was used.
    #[test]
    fn splits_dash_prefixed_fo_flag() {
        let result = apply_filter(
            vec!["-Fodist\\lib.obj".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(
            result,
            vec!["-Fo".to_string(), "dist\\lib.obj".to_string()]
        );
    }

    #[test]
    fn splits_dash_prefixed_fd_flag() {
        let result = apply_filter(
            vec!["-Fdsome\\suspicious\\dirname".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(
            result,
            vec!["-Fd".to_string(), "some\\suspicious\\dirname".to_string()]
        );
    }

    #[test]
    fn splits_mixed_slash_and_dash_flags() {
        let result = apply_filter(
            vec![
                "/Foout\\dir\\a.obj".to_string(),
                "-Fdbuild\\pdb".to_string(),
            ],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(
            result,
            vec![
                "/Fo".to_string(),
                "out\\dir\\a.obj".to_string(),
                "-Fd".to_string(),
                "build\\pdb".to_string(),
            ]
        );
    }

    #[test]
    fn bare_dash_fo_passes_through() {
        // A bare flag (no fused value) is left as-is; the following token is its value.
        let result = apply_filter(
            vec!["-Fo".to_string(), "main.cpp".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(result, vec!["-Fo".to_string(), "main.cpp".to_string()]);
    }

    #[test]
    fn skip_split_keeps_fused_flag_together() {
        let mut cfg = default_cfg();
        cfg.skip_split = true;
        let result = apply_filter(
            vec!["/Fdsome\\target\\directory".to_string()],
            &cfg,
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(result, vec!["/Fdsome\\target\\directory".to_string()]);
    }

// ---- Remove-bad step --------------------------------------------------

    #[test]
    fn removes_bad_flags() {
        let bad = Regex::new(r"^/bigobj$").unwrap();
        let result = apply_filter(
            vec!["/bigobj".to_string(), "main.cpp".to_string()],
            &default_cfg(),
            &bad,
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(result, vec!["main.cpp".to_string()]);
    }

    #[test]
    fn skip_bad_keeps_flags_that_would_be_removed() {
        let bad = Regex::new(r"^/bigobj$").unwrap();
        let mut cfg = default_cfg();
        cfg.skip_bad = true;
        let result = apply_filter(
            vec!["/bigobj".to_string(), "main.cpp".to_string()],
            &cfg,
            &bad,
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(result, vec!["/bigobj".to_string(), "main.cpp".to_string()]);
    }

    // ---- Swap step --------------------------------------------------------

    #[test]
    fn swaps_matching_flags() {
        let swaps = vec![(Regex::new(r"^/Zi$").unwrap(), "-g".to_string())];
        let result = apply_filter(
            vec!["/Zi".to_string()],
            &default_cfg(),
            &never_match(),
            &swaps,
            &no_extra(),
        );
        assert_eq!(result, vec!["-g".to_string()]);
    }

    #[test]
    fn skip_swap_keeps_original_arg() {
        let swaps = vec![(Regex::new(r"^/Zi$").unwrap(), "-g".to_string())];
        let mut cfg = default_cfg();
        cfg.skip_swap = true;
        let result = apply_filter(
            vec!["/Zi".to_string()],
            &cfg,
            &never_match(),
            &swaps,
            &no_extra(),
        );
        assert_eq!(result, vec!["/Zi".to_string()]);
    }

    // ---- Add step (extra flags) -------------------------------------------

    #[test]
    fn inserts_extra_flags_after_x_language() {
        let extra = "FLAG1 FLAG2".to_string();
        let result = apply_filter(
            vec!["-x".to_string(), "a.c".to_string(), "b".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &extra,
        );
        assert_eq!(
            result,
            vec![
                "-x".to_string(),
                "a.c".to_string(),
                "FLAG1".to_string(),
                "FLAG2".to_string(),
                "b".to_string(),
            ]
        );
    }

    #[test]
    fn skip_add_prevents_extra_flags() {
        let extra = "FLAG1 FLAG2".to_string();
        let mut cfg = default_cfg();
        cfg.skip_add = true;
        let result = apply_filter(
            vec!["-x".to_string(), "a.c".to_string(), "b".to_string()],
            &cfg,
            &never_match(),
            &no_swaps(),
            &extra,
        );
        assert_eq!(
            result,
            vec!["-x".to_string(), "a.c".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn inserts_extra_flags_before_source_without_x() {
        // No `-x`/`--`: flags go at the start of the source list (options region),
        // so they stay compiler options instead of trailing the source file.
        let extra = "FLAG1 FLAG2".to_string();
        let result = apply_filter(
            vec!["-O2".to_string(), "main.cpp".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &extra,
        );
        assert_eq!(
            result,
            vec![
                "-O2".to_string(),
                "FLAG1".to_string(),
                "FLAG2".to_string(),
                "main.cpp".to_string(),
            ]
        );
    }

    #[test]
    fn inserts_extra_flags_before_double_dash() {
        // Match the exact CMake/clang-cl shape: `... /Fo<o> /Fd<d> -c -- src.c`.
        // Flags must land BEFORE `--` so clang-cl parses them as options, not as
        // trailing linker/input arguments after the source.
        let extra = "-D_USE_MATH_DEFINES -w".to_string();
        let result = apply_filter(
            vec![
                "/FoCMakeFiles\\t.dir\\t.c.obj".to_string(),
                "/FdCMakeFiles\\t.dir\\".to_string(),
                "-c".to_string(),
                "--".to_string(),
                "C:\\proj\\t.c".to_string(),
            ],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &extra,
        );
        assert_eq!(
            result,
            vec![
                "/Fo".to_string(),
                "CMakeFiles\\t.dir\\t.c.obj".to_string(),
                "/Fd".to_string(),
                "CMakeFiles\\t.dir\\".to_string(),
                "-c".to_string(),
                "-D_USE_MATH_DEFINES".to_string(),
                "-w".to_string(),
                "--".to_string(),
                "C:\\proj\\t.c".to_string(),
            ]
        );
    }

    #[test]
    fn does_not_insert_extra_flags_for_link_step() {
        // A pure link (objects + -o) must never receive compiler-only flags.
        let extra = "FLAG1".to_string();
        let result = apply_filter(
            vec!["main.obj".to_string(), "-o".to_string(), "app.exe".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &extra,
        );
        assert_eq!(
            result,
            vec!["main.obj".to_string(), "-o".to_string(), "app.exe".to_string()]
        );
    }

    #[test]
    fn does_not_duplicate_extra_flags() {
        let extra = "FLAG1 FLAG2".to_string();
        let result = apply_filter(
            vec!["-x".to_string(), "a.c".to_string(), "FLAG1".to_string(), "b".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &extra,
        );
        assert_eq!(
            result,
            vec![
                "-x".to_string(),
                "a.c".to_string(),
                "FLAG1".to_string(),
                "b".to_string(),
            ]
        );
    }

    #[test]
    fn bare_x_does_not_panic_and_runs() {
        // `-x` with nothing after it must not trigger an out-of-bounds index.
        let extra = "FLAG1".to_string();
        let result = apply_filter(
            vec!["-x".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &extra,
        );
        assert_eq!(result, vec!["-x".to_string(), "FLAG1".to_string()]);
    }

    #[test]
    fn empty_extra_flags_do_nothing() {
        let result = apply_filter(
            vec!["-x".to_string(), "a.c".to_string(), "b".to_string()],
            &default_cfg(),
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(result, vec!["-x".to_string(), "a.c".to_string(), "b".to_string()]);
    }

// ---- skip-all ----------------------------------------------------------

    #[test]
    fn skip_all_flags_disables_every_step() {
        let input_args = vec!["/Fdsome\\dir".to_string(), "/bigobj".to_string()];
        let bad = Regex::new(r"^/bigobj$").unwrap();
        let extra = "FLAG".to_string();
        let mut cfg = default_cfg();
        cfg.skip_split = true;
        cfg.skip_bad = true;
        cfg.skip_swap = true;
        cfg.skip_add = true;
        let result = apply_filter(input_args.clone(), &cfg, &bad, &no_swaps(), &extra);
        assert_eq!(result, input_args);
    }

    // ---- Version fallback -------------------------------------------------

    #[test]
    fn empty_args_append_version() {
        assert_eq!(
            apply_filter(vec![], &default_cfg(), &never_match(), &no_swaps(), &no_extra()),
            vec!["--version".to_string()]
        );
    }

    #[test]
    fn skip_version_on_empty_prevents_auto_version() {
        let mut cfg = default_cfg();
        cfg.skip_version_on_empty = true;
        assert_eq!(
            apply_filter(vec![], &cfg, &never_match(), &no_swaps(), &no_extra()),
            Vec::<String>::new()
        );
    }

    // ---- Response files ---------------------------------------------------

    #[test]
    fn force_response_files_uses_response_file() {
        let _guard = RSP_MUTEX.lock().unwrap();
        let mut cfg = default_cfg();
        cfg.force_response_files = true;
        let result = apply_filter(
            vec!["main.cpp".to_string()],
            &cfg,
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(result.len(), 1);
        let rsp = result[0].strip_prefix('@').unwrap().to_string();
        assert!(rsp.ends_with(".rsp"));
        assert!(std::path::Path::new(&rsp).is_absolute());
        let prefix = RESPONSE_FILE_NAME.strip_suffix(".rsp").unwrap();
        assert!(rsp.contains(prefix), "expected a {RESPONSE_FILE_NAME}-based name, got: {rsp}");
        let _ = std::fs::remove_file(&rsp);
    }

    #[test]
    fn args_over_char_limit_become_response_file() {
        let _guard = RSP_MUTEX.lock().unwrap();
        let mut cfg = default_cfg();
        cfg.args_char_limit = 10;
        let result = apply_filter(
            vec!["main.cpp".to_string(), "with-a-very-long-name.cpp".to_string()],
            &cfg,
            &never_match(),
            &no_swaps(),
            &no_extra(),
        );
        assert_eq!(result.len(), 1);
        let rsp = result[0].strip_prefix('@').unwrap().to_string();
        assert!(rsp.ends_with(".rsp"));
        assert!(std::path::Path::new(&rsp).is_absolute());
        let _ = std::fs::remove_file(&rsp);
    }

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
        let result = apply_filter(args.clone(), &cfg, &never_match(), &no_swaps(), &no_extra());
        assert_eq!(result.len(), 1);
        let rsp_path = result[0].strip_prefix('@').unwrap().to_string();
        assert!(std::path::Path::new(&rsp_path).is_absolute());
        let content = std::fs::read_to_string(&rsp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines, args.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let _ = std::fs::remove_file(&rsp_path);
    }

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
        let result = apply_filter(args.clone(), &cfg, &never_match(), &no_swaps(), &no_extra());
        assert_eq!(result.len(), 1);
        let rsp_path = result[0].strip_prefix('@').unwrap().to_string();
        let content = std::fs::read_to_string(&rsp_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines, args.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let _ = std::fs::remove_file(&rsp_path);
    }

    #[test]
    fn preserves_existing_response_file_arg() {
        let _guard = RSP_MUTEX.lock().unwrap();
        let args = vec!["@explicit.rsp".to_string(), "main.cpp".to_string()];
        let mut cfg = default_cfg();
        cfg.force_response_files = true;
        let result = apply_filter(args.clone(), &cfg, &never_match(), &no_swaps(), &no_extra());
        assert_eq!(result, args);
    }

    // ---- Classification ---------------------------------------------------

    #[test]
    fn llvm_compiler_pack_has_compile_extra_flags() {
        let (bad, _swaps, extra) =
            get_args_filter_pack(&(ExecutableFamily::LLVM, ExecutableKind::COMPILER));
        assert!(bad.is_match("/bigobj"));
        assert_eq!(extra, LLVM_COMPILER_EXTRA_FLAGS);
    }

    #[test]
    fn llvm_linker_manifest_embed_is_replaced_not_dropped() {
        // Regression: deleting `/MANIFEST:EMBED` as a "bad" flag orphaned the
        // preceding `-Xlinker`/`-Wl,` gate, breaking the link command. It must
        // now be swapped in place to `/MANIFEST:NO` so the pair stays intact.
        let (bad, swaps, extra) =
            get_args_filter_pack(&(ExecutableFamily::LLVM, ExecutableKind::LINKER));
        assert!(!bad.is_match("/MANIFEST:EMBED"), "manifest must be swapped, not dropped");
        let result = apply_filter(
            vec!["-Xlinker".to_string(), "/MANIFEST:EMBED".to_string()],
            &default_cfg(),
            &bad,
            &swaps,
            &extra.to_string(),
        );
        assert_eq!(
            result,
            vec!["-Xlinker".to_string(), "/MANIFEST:NO".to_string()]
        );
    }

    #[test]
    fn llvm_linker_manifest_variants_are_swapped_to_no() {
        let (bad, swaps, extra) =
            get_args_filter_pack(&(ExecutableFamily::LLVM, ExecutableKind::LINKER));
        for input in ["MANIFEST:EMBED", "/MANIFEST:EMBED", "/MANIFEST:EMBED,ID=2"] {
            let result = apply_filter(
                vec![input.to_string()],
                &default_cfg(),
                &bad,
                &swaps,
                &extra.to_string(),
            );
            assert_eq!(result, vec!["/MANIFEST:NO".to_string()], "for input {input}");
        }
    }

    #[test]
    fn llvm_linker_incremental_no_still_dropped() {
        // `/INCREMENTAL:NO` is still removed as a bad flag; it is typically a
        // self-standing linker arg, not the value of an -Xlinker gate.
        let (bad, swaps, extra) =
            get_args_filter_pack(&(ExecutableFamily::LLVM, ExecutableKind::LINKER));
        let result = apply_filter(
            vec!["/INCREMENTAL:NO".to_string(), "main.obj".to_string()],
            &default_cfg(),
            &bad,
            &swaps,
            &extra.to_string(),
        );
        assert_eq!(result, vec!["main.obj".to_string()]);
    }

    #[test]
    fn llvm_linker_pack_has_linker_extra_flags() {
        let (_bad, _swaps, extra) =
            get_args_filter_pack(&(ExecutableFamily::LLVM, ExecutableKind::LINKER));
        assert_eq!(extra, LLVM_LINKER_EXTRA_FLAGS);
        assert!(!extra.contains("-w"), "linker extra flags must not be compiler-only");
    }
}