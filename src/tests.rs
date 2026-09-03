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
    let mut args = vec!["main.cpp".to_string()];
    let (wrapper, tool) = get_executable_names(&"myccache-gcc".to_string(), &mut args);
    assert_eq!(wrapper, "myccache");
    assert_eq!(tool, "gcc");
    assert_eq!(args, vec!["main.cpp"]);
}

#[test]
fn sccache_prefixed_names_split_into_driver_and_tool() {
    for (full, expect_wrapper, expect_tool) in [
        ("sccache-clang", "sccache", "clang"),
        ("sccache-clang-cl", "sccache", "clang-cl"),
        ("sccache-g++", "sccache", "g++"),
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
    let result = apply_filter(input_args, &default_cfg(), &never_match(), &no_swaps(), &no_extra(), ExecutableFamily::MSVC);
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
    let result = apply_filter(input_args, &default_cfg(), &never_match(), &no_swaps(), &no_extra(), ExecutableFamily::MSVC);
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
            ExecutableFamily::MSVC,
        ),
        vec!["/Fo".to_string(), "dist\\lib.obj".to_string()]
    );
}

// ---- Bad flag removal ---------------------------------------------------

#[test]
fn drops_bad_flags() {
    let bad = Regex::new(r"(?i)(-m[\w-]+|-f[\w-]+)").unwrap();
    let result = apply_filter(
        vec![
            "-m64".to_string(),
            "-O2".to_string(),
            "-ffast-math".to_string(),
            "a.c".to_string(),
        ],
        &default_cfg(),
        &bad,
        &no_swaps(),
        &no_extra(),
        ExecutableFamily::LLVM,
    );
    assert_eq!(result, vec!["-O2".to_string(), "a.c".to_string()]);
}

#[test]
fn drops_bad_flags_with_value() {
    let bad = Regex::new(r"(?i)(-march=[\w-]+|-mtune=[\w-]+)").unwrap();
    let result = apply_filter(
        vec![
            "-march=native".to_string(),
            "-O2".to_string(),
            "-mtune=generic".to_string(),
            "a.c".to_string(),
        ],
        &default_cfg(),
        &bad,
        &no_swaps(),
        &no_extra(),
        ExecutableFamily::LLVM,
    );
    assert_eq!(result, vec!["-O2".to_string(), "a.c".to_string()]);
}

// ---- Swap pairs ---------------------------------------------------------

#[test]
fn swaps_stdlib_for_llvm() {
    let swap = vec![(
        Regex::new(r"(?i)-stdlib=libstdc\+\+").unwrap(),
        "-stdlib=libc++".to_string(),
    )];
    let result = apply_filter(
        vec![
            "-stdlib=libstdc++".to_string(),
            "-O2".to_string(),
            "a.c".to_string(),
        ],
        &default_cfg(),
        &never_match(),
        &swap,
        &no_extra(),
        ExecutableFamily::LLVM,
    );
    assert_eq!(
        result,
        vec!["-stdlib=libc++".to_string(), "-O2".to_string(), "a.c".to_string()]
    );
}

// ---- Extra flag insertion -----------------------------------------------

#[test]
fn inserts_extra_flags_before_source() {
    let extra = "-D_FOO -D_BAR".to_string();
    let result = apply_filter(
        vec!["-O2".to_string(), "a.c".to_string()],
        &default_cfg(),
        &never_match(),
        &no_swaps(),
        &extra,
        ExecutableFamily::LLVM,
    );
    assert_eq!(
        result,
        vec!["-D_FOO".to_string(), "-D_BAR".to_string(), "-O2".to_string(), "a.c".to_string()]
    );
}

#[test]
fn inserts_extra_flags_after_value_flags() {
    let extra = "-D_FOO".to_string();
    let result = apply_filter(
        vec![
            "-I".to_string(),
            "include".to_string(),
            "-L".to_string(),
            "lib".to_string(),
            "-l".to_string(),
            "m".to_string(),
            "a.c".to_string(),
        ],
        &default_cfg(),
        &never_match(),
        &no_swaps(),
        &extra,
        ExecutableFamily::LLVM,
    );
    assert_eq!(
        result,
        vec![
            "-I".to_string(),
            "include".to_string(),
            "-L".to_string(),
            "lib".to_string(),
            "-D_FOO".to_string(),
            "-l".to_string(),
            "m".to_string(),
            "a.c".to_string(),
        ]
    );
}

#[test]
fn bare_value_flag_at_end_gets_extra_after() {
    // `-x` with nothing after it must not trigger an out-of-bounds index.
    let extra = "FLAG1".to_string();
    let result = apply_filter(
        vec!["-x".to_string()],
        &default_cfg(),
        &never_match(),
        &no_swaps(),
        &extra,
        ExecutableFamily::LLVM,
    );
    assert_eq!(result, vec!["FLAG1".to_string(), "-x".to_string()]);
}

#[test]
fn empty_extra_flags_do_nothing() {
    let result = apply_filter(
        vec!["-x".to_string(), "a.c".to_string(), "b".to_string()],
        &default_cfg(),
        &never_match(),
        &no_swaps(),
        &no_extra(),
        ExecutableFamily::LLVM,
    );
    assert_eq!(result, vec!["-x".to_string(), "a.c".to_string(), "b".to_string()]);
}

#[test]
fn mt_flag_value_not_split_by_extra_flags() {
    // Regression test: -MT takes a value (the dependency rule target).
    // Extra flags must be inserted AFTER the -MT value, not between -MT and its value.
    // This was the bug that broke CMake's compiler detection in configure.log.
    let extra = "-D_USE_MATH_DEFINES -D_CRT_SECURE_NO_WARNINGS -w -Wno-everything".to_string();
    let result = apply_filter(
        vec![
            "-D_MBCS".to_string(),
            "-O3".to_string(),
            "-DNDEBUG".to_string(),
            "-D_DLL".to_string(),
            "-D_MT".to_string(),
            "-Xclang".to_string(),
            "--dependent-lib=msvcrt".to_string(),
            "-MD".to_string(),
            "-MT".to_string(),
            "CMakeFiles/cmTC_412a9.dir/testCCompiler.c.obj".to_string(),
            "-MF".to_string(),
            "CMakeFiles\\cmTC_412a9.dir\\testCCompiler.c.obj.d".to_string(),
            "-o".to_string(),
            "CMakeFiles/cmTC_412a9.dir/testCCompiler.c.obj".to_string(),
            "-c".to_string(),
            "C:/Dev/Projects/TheRock/build/CMakeFiles/CMakeScratch/TryCompile-babef3/testCCompiler.c".to_string(),
        ],
        &default_cfg(),
        &never_match(),
        &no_swaps(),
        &extra,
        ExecutableFamily::LLVM,
    );
    // The extra flags must appear immediately BEFORE the last flag (-c),
    // not between -MT and its value, nor between -MF and its value.
    assert_eq!(
        result,
        vec![
            "-D_MBCS".to_string(),
            "-O3".to_string(),
            "-DNDEBUG".to_string(),
            "-D_DLL".to_string(),
            "-D_MT".to_string(),
            "-Xclang".to_string(),
            "--dependent-lib=msvcrt".to_string(),
            "-MD".to_string(),
            "-MT".to_string(),
            "CMakeFiles/cmTC_412a9.dir/testCCompiler.c.obj".to_string(),
            "-MF".to_string(),
            "CMakeFiles\\cmTC_412a9.dir\\testCCompiler.c.obj.d".to_string(),
            "-o".to_string(),
            "CMakeFiles/cmTC_412a9.dir/testCCompiler.c.obj".to_string(),
            "-D_USE_MATH_DEFINES".to_string(),
            "-D_CRT_SECURE_NO_WARNINGS".to_string(),
            "-w".to_string(),
            "-Wno-everything".to_string(),
            "-c".to_string(),
            "C:/Dev/Projects/TheRock/build/CMakeFiles/CMakeScratch/TryCompile-babef3/testCCompiler.c".to_string(),
        ]
    );
}

#[test]
fn inserts_extra_flags_before_object_file() {
    let extra = "-D_FOO".to_string();
    let result = apply_filter(
        vec![
            "-I".to_string(),
            "include".to_string(),
            "a.o".to_string(),
        ],
        &default_cfg(),
        &never_match(),
        &no_swaps(),
        &extra,
        ExecutableFamily::LLVM,
    );
    assert_eq!(
        result,
        vec![
            "-D_FOO".to_string(),
            "-I".to_string(),
            "include".to_string(),
            "a.o".to_string(),
        ]
    );
}

#[test]
fn passes_clang_prefixed_and_splits_fd() {
    let input_args = vec![
        "-clang:-MSmy\\target\\dir.name\\myname\\".to_string(),
        "-clang:/FoCMakeLists\\my\\sussy.dir\\".to_string(),
        "/Fdsome\\suspicious\\dirname".to_string(),
    ];
    let result = apply_filter(input_args, &default_cfg(), &never_match(), &no_swaps(), &no_extra(), ExecutableFamily::LLVM);
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

#[test]
fn splits_dash_prefixed_fo_flag() {
    let result = apply_filter(
        vec!["-Fodist\\lib.obj".to_string()],
        &default_cfg(),
        &never_match(),
        &no_swaps(),
        &no_extra(),
        ExecutableFamily::MSVC,
    );
    assert_eq!(
        result,
        vec!["-Fo".to_string(), "dist\\lib.obj".to_string()]
    );
}