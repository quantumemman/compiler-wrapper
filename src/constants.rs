use std::env;
use std::sync::LazyLock;
use regex::Regex;

/////////////////////////////////////////////////////////////////////////////////////////
//                         Define Executables Search Paths                             //
/////////////////////////////////////////////////////////////////////////////////////////
// Microsoft Visual C++ toolchain path
pub const MSVC_PATH: &str = env!("MSVC_PATH_");
// Visual Studio LLVM toolchain path
pub const LLVM_PATH_VS: &str = env!("LLVM_PATH_VS_");
// Custom LLVM toolchain path
pub const LLVM_PATH: &str = env!("LLVM_PATH_");
// GNU Compiler Collection path
pub const GCC_PATH: &str = env!("GCC_PATH_");
// Path to wrappers such as sccache and ccache
pub const WRAPPER_PATH: &str = env!("PY_PATH_");

/// Returns the ordered list of executable search paths.
/// If `WRAPPER_PREFER_VS` is set, prefers Visual Studio's LLVM toolchain;
/// otherwise prefers Custom LLVM.
pub static PATHS: LazyLock<[&str; 5]> = LazyLock::new(|| {
    if env::var("WRAPPER_PREFER_VS").is_ok() {
        log::info!("Preferring Visual Studio LLVM toolchain");
        [LLVM_PATH_VS, MSVC_PATH, LLVM_PATH, GCC_PATH, WRAPPER_PATH]
    } else {
        log::debug!("Preferring Custom LLVM toolchain");
        [LLVM_PATH, LLVM_PATH_VS, MSVC_PATH, GCC_PATH, WRAPPER_PATH]
    }
});

/////////////////////////////////////////////////////////////////////////////////////////
//                     Define Wrapper/Compiler/Linker Keywords                         //
/////////////////////////////////////////////////////////////////////////////////////////
// Keywords for detecting wrapper executables (e.g., ccache)
pub static EXTERNAL_WRAPPER_KEYWORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)ccache"#).unwrap());

// Compiler executable keywords
pub static COMPILER_KEYWORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(clang|cl|gcc|g\+\+)"#).unwrap());

// Linker executable keywords
pub static LINKER_KEYWORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(link|lld)"#).unwrap());

// LLVM family keywords
pub static LLVM_KEYWORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(clang|lld-link)"#).unwrap());

// MSVC family keywords
pub static MSVC_KEYWORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(cl|link)"#).unwrap());

// GCC family keywords
pub static GCC_KEYWORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(gcc|g\+\+|ld)"#).unwrap());

// Matches executable names that are wrappers from this project like clang-rs
// to avoid doing their work for them since they will be called by this program
pub static PROJECT_SIGNATURE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)[-]rs"#).unwrap());

/////////////////////////////////////////////////////////////////////////////////////////
//                                Define Bad Flag Regexes                              //
/////////////////////////////////////////////////////////////////////////////////////////
pub static LLVM_COMPILER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^([-/](clang:))?[-/](permissive-|(D[-/])?bigobj|EGR|W3|W4|Wc\+\+11-narrowing|Wincompatible-pointer-types|Wimplicit-function-declaration|Wdeprecated-declarations|Wextern-initializer|Wold-style-cast|Wunused-variable|Wunused-function|Wunused-command-line-argument|Wlogical-op-parentheses|Wignored-attributes|Wunknown-warning-option)$"#).unwrap()});
pub static MSVC_COMPILER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^([-/](clang:))?[-/]((D[-/])?bigobj|GR|Od|W3|W4|Wc\+\+11-narrowing|Wincompatible-pointer-types|Wimplicit-function-declaration|Wdeprecated-declarations|Wextern-initializer|Wold-style-cast|Wunused-variable|Wunused-function|Wunused-command-line-argument|Wlogical-op-parentheses|Wignored-attributes|Wunknown-warning-option)$"#).unwrap()});
pub static GCC_COMPILER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^[-/](Werror|ffast-math|fstrict-aliasing|fpack-struct|fshort-enum)"#).unwrap()});
pub static LLVM_LINKER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^[-/](Werror)"#).unwrap()});
pub static MSVC_LINKER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^[-/](Werror)"#).unwrap()});
pub static GCC_LINKER_BAD_FLAGS: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^[-/](Werror)"#).unwrap()});

// Flags whose value are fused with the prefix (e.g. `/Fdsome\dir`, `/Foout.obj`).
// When the split-flags feature is enabled, these get split into the prefix and the value e.g. `/Fd` and `some\dir`.
pub static SPLIT_FUSED_FLAGS: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^[-/](Fd|Fo)"#).unwrap()});

// Flags that need their prefix fixed (e.g. `/version:0.0` -> `-version:0.0`)
pub static FIX_FLAG_PREFIXES: LazyLock<Regex> = LazyLock::new(|| {Regex::new(r#"^(?i)[/](version:)"#).unwrap()});

/////////////////////////////////////////////////////////////////////////////////////////
//                            Swap pairs per classification                            //
/////////////////////////////////////////////////////////////////////////////////////////

pub static MSVC_COMPILER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^[-/]Zi$").expect(BAD_MATCH_MESSAGE), "/Z7".into()),
    ]
});

pub static LLVM_COMPILER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^[-/]Z(i|7)$").expect(BAD_MATCH_MESSAGE), "-g".into()),
        (Regex::new(r"^[/]LTCG$").expect(BAD_MATCH_MESSAGE), "-flto".into()),
        (Regex::new(r"^[-/]O1$").expect(BAD_MATCH_MESSAGE), "-O1".into()),
        (Regex::new(r"^[-/]O2$").expect(BAD_MATCH_MESSAGE), "-O2".into()),
        (Regex::new(r"^[-/]O3$").expect(BAD_MATCH_MESSAGE), "-O3".into()),
        (Regex::new(r"^[-/]O4$").expect(BAD_MATCH_MESSAGE), "-O4".into()),
    ]
});

pub static GCC_COMPILER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^[-/]Z(i|7)$").expect(BAD_MATCH_MESSAGE), "-g".into()),
    ]
});

pub static MSVC_LINKER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^[-/]flto$").expect(BAD_MATCH_MESSAGE), "/LTCG".into()),
        (Regex::new(r"^[/]INCREMENTAL(:YES)?$").expect(BAD_MATCH_MESSAGE), "/INCREMENTAL:NO".into()),
        // (Regex::new(r"^[/]MANIFEST:EMBED(,ID=\d+)?$").expect(BAD_MATCH_MESSAGE), "/MANIFEST:NO".into()),
    ]
});

pub static LLVM_LINKER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BAD_MATCH_MESSAGE), "-flto".into()),
    ]
});

pub static GCC_LINKER_SWAP_PAIRS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^[/]LTCG$").expect(BAD_MATCH_MESSAGE), "-flto".into()),
    ]
});

/////////////////////////////////////////////////////////////////////////////////////////
//                                  Define Extra Flags                                 //
/////////////////////////////////////////////////////////////////////////////////////////
pub const MSVC_COMPILER_EXTRA_FLAGS: &str = "-D_USE_MATH_DEFINES -D_CRT_SECURE_NO_WARNINGS -D_CRT_SECURE_NO_DEPRECATE -D_CRT_NONSTDC_NO_DEPRECATE -w -W0";
pub const LLVM_COMPILER_EXTRA_FLAGS: &str = "-D_USE_MATH_DEFINES -D_CRT_SECURE_NO_WARNINGS -D_CRT_SECURE_NO_DEPRECATE -D_CRT_NONSTDC_NO_DEPRECATE -w -Wno-everything";
pub const GCC_COMPILER_EXTRA_FLAGS: &str = "-D_CRT_SECURE_NO_WARNINGS -D_CRT_SECURE_NO_DEPRECATE -D_CRT_NONSTDC_NO_DEPRECATE -w";
pub const MSVC_LINKER_EXTRA_FLAGS: &str = "/LTCG";
pub const LLVM_LINKER_EXTRA_FLAGS: &str = "-flto";
pub const GCC_LINKER_EXTRA_FLAGS: &str = "-flto";

/////////////////////////////////////////////////////////////////////////////////////////
//                                 General Constants                                   //
// //////////////////////////////////////////////////////////////////////////////////////
/// Maximum CLI args char length before using response file
pub const ARGS_CHAR_LIMIT: usize = 30000;
/// Unknown keyword placeholder
pub const UNKNOWN_KEYWORD: &str = "UNKNOWN";
/// Bad match message for regex construction errors
pub const BAD_MATCH_MESSAGE: &str = "bad match";
/// Response file name pattern (uses process ID)
pub const RESPONSE_FILE_NAME: &str = "@wrapper_<pid>.rsp";

/////////////////////////////////////////////////////////////////////////////////////////
//                              CLI Flag Constants                                     //
/////////////////////////////////////////////////////////////////////////////////////////
/// Short help flag
pub const CLI_FLAG_HELP_SHORT: &str = "-h";
/// Long help flag
pub const CLI_FLAG_HELP_LONG: &str = "--help";
/// Usage flag
pub const CLI_FLAG_USAGE: &str = "--usage";
/// Short version flag
pub const CLI_FLAG_VERSION_SHORT: &str = "-v";
/// Long version flag
pub const CLI_FLAG_VERSION_LONG: &str = "--version";
