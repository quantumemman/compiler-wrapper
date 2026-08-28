use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{id};
use std::string::String;
use std::sync::LazyLock; // for LazyLock init
use regex::Regex;
use log::{trace, debug, info}; // import the logging macros. Options include trace, debug, info, warn, error
pub const ARGS_CHAR_LIMIT: usize = 30000;

////////////////////////////////////////////////////////////////////////////////////////
//                                  Print Usage Help                                  //
////////////////////////////////////////////////////////////////////////////////////////
/// Prints the wrapper usage/help message when `WRAPPER_OPTIONS` or `WRAPPER_HELP` is set.
pub fn printUsage() {
    if env::var("WRAPPER_OPTIONS").is_ok() || env::var("WRAPPER_HELP").is_ok() {
        println!("WRAPPER_PREFER_VS: Prefer VS Studio LLVM executables over ROCm LLVM.");
        println!("WRAPPER_SKIP_BAD_FLAGS: Skip removing bad flags.");
        println!("WRAPPER_SKIP_SWAP_FLAGS: Skip swapping problematic flags.");
        println!("WRAPPER_SKIP_ADD_FLAGS: Skip adding extra helpful flags.");
        println!("WRAPPER_SKIP_SPLIT_FLAGS: Skip splitting fused flags.");
        println!("WRAPPER_SKIP_ALL_FLAGS: Skip removing bad flags, swapping problematic flags, adding extra helpful flags, and splitting fused flags.");
        println!("WRAPPER_SKIP_VERSION_HANDLING: Skip CLI version handling: -v, --version.");
        println!("WRAPPER_ARGS_CHAR_LIMIT: Override the default argument character limit of {} to enable response files.", ARGS_CHAR_LIMIT);
        println!("WRAPPER_FORCE_RESPONSE_FILES: Force response files regardless of arguments total char length of {}", ARGS_CHAR_LIMIT);
        println!("WRAPPER_ENABLE_PASSTHROUGH: Pass through arguments directly without processing.");
        println!("WRAPPER_OPTIONS or WRAPPER_HELP: Print this help message and exit.");
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
//                                   Filter arguments                                  //
/////////////////////////////////////////////////////////////////////////////////////////
pub fn filterArgs(args: Vec<String>, BadFlags: Regex, SwapPairs: Vec<(Regex, String)>, ExtraFlags: String) -> Vec<String> {
    // Split fused flag+directory args (e.g. /Fdsome\target\directory -> /Fd + some\target\directory)
    // BEFORE the swap/bad loop, because one fused arg expands into two and the array grows.
    // An arg that is exactly the flag (already standalone) passes through unchanged.
    let mut expanded: Vec<String> = Vec::new();
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

    // final arguments
    let mut finalArgs: Vec<String> = Vec::new();
    for arg in expanded {
        if BadFlags.is_match(&arg) { continue; }       // Drop if bad
        else {
            let mut newArg = arg.clone();              // Swap if match
            for (re, swap) in &*SwapPairs {
                if re.is_match(&arg) {
                    newArg = re.replace(&arg, swap.clone()).trim().to_string();
                    break; }}
            if !newArg.is_empty() {
                finalArgs.push(newArg); }
        }
    }
    
    if finalArgs.is_empty() { 
        finalArgs.push("--version".into());
    } else if finalArgs.len() > 1 && !ExtraFlags.is_empty() && finalArgs.iter().any(|a| a == "-x") {
        let index = finalArgs.iter().position(|a| a == "-x").unwrap();
        if finalArgs[index + 2] != ExtraFlags.split(" ").next().unwrap() {  // Avoid inserting extra flags twice
            finalArgs.splice(index + 2..index + 2, ExtraFlags.split(" ").map(|s| s.into())); }
    }

    if finalArgs.iter().any(|a| a.starts_with('@')) {
        // rsp already used—pass through
    } else if finalArgs.join(" ").len() > ARGS_CHAR_LIMIT { // too long: make rsp
        let rsp_path = &ResponseFileName.replace(".rsp", &format!("_{}.rsp", id()).to_owned());
        let mut f = File::create(&rsp_path).unwrap();
        for arg in &finalArgs { writeln!(f, "{}", arg).unwrap(); }  // or space/newline
        finalArgs = vec![format!("@{}", &rsp_path)]; }
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

//////////////////////////////////////////////////////////////////////////////////////////
//                                   Cargo Tests                                        //
//////////////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;

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
}
