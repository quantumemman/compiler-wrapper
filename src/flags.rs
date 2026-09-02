use crate::classification::ExecutableFamily;

/// Flags that take a separate value as the next argument, for LLVM/Clang.
pub const LLVM_FLAGS_WITH_VALUE: &[&str] = &[
    // Include paths
    "-I", "/I",
    // Library paths and libraries
    "-L", "/L", "-l",
    // Defines
    "-D", "/D",
    // Output files
    "-o",
    // Language specification
    "-x",
    // Preprocessor flags
    "-Xpreprocessor", "-include", "-imacros",
    // System root and include paths
    "-isysroot", "-isystem", "-idirafter", "-iprefix",
    "-iwithprefix", "-iwithprefixbefore",
    // Linker flags
    "-Xlinker",
    // Clang-specific
    "-Xclang",
    // Standard specification
    "-std",
    // Profile-guided optimization
    "-fprofile-generate", "-fprofile-use",
    // Coverage
    "-coverage",
    // Dependency rule target/output (Clang/GCC)
    "-MT", "-MF", "-MQ",
];

/// Flags that take a separate value as the next argument, for MSVC.
pub const MSVC_FLAGS_WITH_VALUE: &[&str] = &[
    // Include paths
    "-I", "/I",
    // Library paths and libraries
    "-L", "/L", "-l",
    // Defines
    "-D", "/D",
    // Output files
    "-o", "-Fo", "/Fo", "-Fd", "/Fd", "-Fe", "/Fe",
    "-Fa", "/Fa", "-Fm", "/Fm", "-Fp", "/Fp",
    "-FR", "/FR", "-FU", "/FU",
    // Language specification
    "-x",
    // Linker flags
    "-Xlinker",
    // NOTE: -MT, -MF are NOT here — in MSVC they are standalone flags
];

/// Flags that take a separate value as the next argument, for GCC.
pub const GCC_FLAGS_WITH_VALUE: &[&str] = &[
    // Include paths
    "-I", "/I",
    // Library paths and libraries
    "-L", "/L", "-l",
    // Defines
    "-D", "/D",
    // Output files
    "-o",
    // Language specification
    "-x",
    // Preprocessor flags
    "-Xpreprocessor", "-include", "-imacros",
    // System root and include paths
    "-isysroot", "-isystem", "-idirafter", "-iprefix",
    "-iwithprefix", "-iwithprefixbefore",
    // Linker flags
    "-Xlinker",
    // Standard specification
    "-std",
    // Profile-guided optimization
    "-fprofile-generate", "-fprofile-use",
    // Coverage
    "-coverage",
    // Dependency rule target/output (Clang/GCC)
    "-MT", "-MF", "-MQ",
];

/// Returns the list of flags that take a separate value argument,
/// for the given compiler family.
pub fn flags_with_value(executable_family: ExecutableFamily) -> &'static [&'static str] {
    match executable_family {
        ExecutableFamily::LLVM => LLVM_FLAGS_WITH_VALUE,
        ExecutableFamily::MSVC => MSVC_FLAGS_WITH_VALUE,
        ExecutableFamily::GCC => GCC_FLAGS_WITH_VALUE,
        ExecutableFamily::Unknown => LLVM_FLAGS_WITH_VALUE,
    }
}
