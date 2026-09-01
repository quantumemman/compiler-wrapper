# WRAPPER — Compiler Argument Wrapper Helper

A dependency-light Rust wrapper for C/C++ toolchain executables (Clang/LLVM,
MSVC, GCC, GNU `ld`) and their `sccache` variants, geared toward Windows. Each
wrapper:

1. **Locates** the real underlying toolchain executable from a list of install
   paths baked in at compile time.
2. **Cleans / rewrites** command-line flags so arguments authored for one
   toolchain work on another (e.g. MSVC flags fed to LLVM Clang).
3. **Dispatches** to the located executable with the rewritten arguments,
   forwarding its exit status.

The wrappers are thin binaries that all share the same logic in `src/lib.rs`
(the `wrapper` library crate). Each binary's `main` is a few lines: initialize
the logger, optionally print the usage/help banner, build a `Runtime`, then
spawn the target tool with the processed arguments.

---

## Project layout

```
wrapper/
├── Cargo.toml        # crate manifest; defines every [[bin]] wrapper
├── src/
│   ├── lib.rs        # all shared logic (path lookup, classification, filtering)
│   └── *.rs          # one near-identical main per wrapper binary
├── bin/              # (optional) deployed/copied wrapper executables
└── target/           # cargo build output (git-ignored)
```

### Binaries

| Wrapper (`cargo build` target) | Underlying tool it dispatches to |
|--------------------------------|----------------------------------|
| `cl-rs`                  | MSVC C/C++ compiler (`cl`) |
| `clang-rs`               | Clang C compiler |
| `clangpp-rs`             | Clang++ C++ compiler |
| `clang-cl-rs`            | Clang in MSVC-compatible mode (`clang-cl`) |
| `gcc-rs`                 | GNU GCC |
| `gpp-rs`                 | GNU G++ |
| `hipcc-rs`               | HIP compile driver |
| `link-rs`                | MSVC linker (`link`) |
| `lld-link-rs`            | LLVM `lld-link` |
| `ld-rs`                  | GNU `ld` |
| `sccache-rs`             | `sccache` (pass-through driver) |
| `sccache-clang-rs`, `sccache-clangpp-rs`, `sccache-clang-cl-rs` | `sccache` + Clang family |
| `sccache-gcc-rs`, `sccache-gpp-rs` | `sccache` + GCC/G++ |
| `sccache-hipcc-rs`       | `sccache` + HIP |

The `-rs` suffix distinguishes the wrapper executables from the real tools they
call (e.g. `clang-rs.exe` → invokes `clang.exe`).

---

## Building

Requires a Rust toolchain with the 2024 edition (Rust 1.85+).

```sh
cargo build                 # debug build of every wrapper binary
cargo build --release       # release build
```

The set of install paths used to locate the real toolchains is baked in **at
compile time** from environment variables, so they must be set when building:

| Build-time variable | Purpose |
|---------------------|---------|
| `NVCC_PATH_`    | NVIDIA CUDA compiler directory |
| `LLVM_PATH_VS_` | Visual Studio's bundled LLVM (Clang at `LLVM/../../VC/Tools/Llvm`) |
| `MSVC_PATH_`    | MSVC toolchain root |
| `LLVM_PATH_`    | ROCm / standalone LLVM install root |
| `GCC_PATH_`     | GCC install root |
| `PY_PATH_`      | Python venv `Scripts` directory |

> These trailing-underscore names are read via the `env!()` macro, i.e. they are
> expanded when the crate is compiled, not at runtime. They form the search
> order used later when locating executables.

---
## Runtime behaviour

Print the full usage/help banner (also lists every runtime variable):

```sh
set WRAPPER_HELP=1    # PowerShell: $env:WRAPPER_HELP="1"
clang-rs.exe          # prints the help box and exits
```

`WRAPPER_OPTIONS` has the same effect as `WRAPPER_HELP`.

Each wrapper's `main` collects `env::args().skip(1)`, builds a `Runtime`, then
runs the real tool:

```rust
let runtime = Runtime::new(file!().to_string(), input_args);
let status = Command::new(&runtime.main_exe)
    .args(&runtime.final_args)
    .status()
    .expect(&runtime.expect);
```

### Executable resolution

- The wrapper's own name (e.g. `clang-cl-rs`) is derived from its source file.
- `get_executable_names` decides what to invoke. If the name contains a wrapper
  keyword (`ccache`, matched case-insensitively, so it catches `sccache`,
  `ccache`, etc.):
  - a combined `<wrapper>-<tool>` name (e.g. `sccache-clang-cl`, `ccache-g++`)
    splits at the first `-`: the part before is the driver (`sccache`),
    the part after is the deputy tool (`clang-cl`) — no hardcoded list of
    wrapper names needed.
  - a bare `ccache` / `sccache` wrapper consumes the *first argument* as the real
    tool, since pure drivers are neither compilers nor linkers.
- `get_executable_paths` searches the baked-in `PATHS` (ordered by
  `WRAPPER_PREFER_VS`) for each name, appending `.exe` on Windows; absolute
  paths and other `*-rs` wrappers are returned as-is.
- The **main** executable is the wrapper driver (or the tool itself when no
  wrapper is involved); the **deputy** is the right-most real tool, which also
  drives classification.

### Argument pipeline

Unless a step is disabled (or `WRAPPER_ENABLE_PASSTHROUGH` is set), each wrapper
rewrites the argument list through these stages:

1. **Split** fused `/Fd<dir>` / `/Fo<dir>` flags into the bare flag plus the
   directory value (the original `/` or `-` prefix is preserved). A bare flag
   with no value passes through unchanged. **This step is OFF by default** —
   set `WRAPPER_SPLIT_FLAGS` to enable it.
2. **Remove bad flags** — drop arguments matching the family/kind's bad-flag
   set (e.g. MSVC-specific `-Wno-*` warning suppressions that clang would choke
   on, or `/INCREMENTAL:NO` on linkers).
3. **Swap problematic flags** — replace a flag with a portable equivalent
   (`/MD` / `/MDd` → `-fms-extensions`, `/Zi` → `-g` on LLVM/GCC or `/Z7` on
   MSVC, `/LTCG` → `-flto`, `/O1`–`/O4` → `-O1`–`-O4`). A pair whose replacement
   is empty removes the argument.
4. **Add helpful flags** — splice compiler/toolchain extras (e.g.
   `-D_USE_MATH_DEFINES`, warning suppressions, `/MANIFEST:NO`) into the
   *options* region of the command without ever splitting a flag/value pair.
   Placement, in priority order:
   - right after a `-x <lang>` token (bounds-checked so a trailing `-x` cannot panic),
   - just before a `--` separator (everything after it is positional),
   - before the first source file,
   - only at the end for a clear compile step; a pure link (objects + `-o`) is
     left untouched so compiler-only flags never leak into a link.
   Already-present tokens are deduplicated.
5. **Version fallback** — if the argument list ends up empty, a lone `--version`
   is appended so the tool still prints something useful. This is mutually
   exclusive with step 4 / any response-file content.
6. **Response files** — if any argument already starts with `@`, the list is
   passed through untouched. Otherwise, when the joined arguments exceed
   `WRAPPER_ARGS_CHAR_LIMIT` (default `30000`) or `WRAPPER_FORCE_RESPONSE_FILES`
   is set, the arguments are written to an absolute `@<pid>.rsp` file in the
   system temp directory and the list collapses to a single `@file` argument.

### Classification

The exact bad/swap/extra flags are chosen by classifying the target deputy
executable from its name:

* **Family** — `clang-cl` is treated as MSVC (checked first), else `LLVM`
  (`clang|llvm|lld`), `MSVC` (`cl|link`), `GCC` (`gcc|g++|ld`), or `UNKNOWN`.
* **Kind** — `COMPILER` (`clang|hip|cl|gcc|g++`) is prioritized over `LINKER`
  (`link|lld|ld`), else `UNKNOWN` / `WRAPPER` (`sccache`, `ccache`).

Each (family, kind) pair maps to a filter pack:
| Pack | Bad flags (excerpt) | Swaps (excerpt) | Extra flags |
|------|---------------------|-----------------|-------------|
| LLVM compiler | `/EHsc`, `permissive-`, `bigobj`, `EGR`, `W3`, several `-Wno-*` | `/MD{,d}`→`-fms-extensions`, `/Zi`→`-g`, `/O1`–`/O4`→`-O1`–`-O4` | `-D_USE_MATH_DEFINES`, `-D_CRT_SECURE_NO_WARNINGS`, `-w` |
| LLVM linker | `/INCREMENTAL:NO` | `/LTCG`→`-flto`, `/MANIFEST:EMBED{,ID=2}`→`/MANIFEST:NO` | `/MANIFEST:NO` |
| MSVC compiler | `bigobj`, `GR`, `Od`, `W3`, several `-Wno-*` | `/Zi`→`/Z7` | `-D_USE_MATH_DEFINES` `-D_CRT_SECURE_NO_WARNINGS` `-FS` `-w` |
| MSVC linker | `/INCREMENTAL:NO` | `/LTCG`→`-flto`, `/MANIFEST:EMBED{,ID=2}`→`/MANIFEST:NO` | *(none)* |
| GCC compiler | `/Werror`, `ffast-math`, `fstrict-aliasing`, `fpack-struct`, `fshort-enum` | `/Zi`→`-g` | `-w` |
| GCC linker | `/Werror` | `/LTCG`→`-flto` | *(none)* |

### Logging (`RUST_LOG`)

Logging is provided by `env_logger` via the standard `RUST_LOG` variable (unset
⇒ `error`). Higher values print wrapper diagnostics such as the located
executable, family/kind classification and the final rewritten arguments:

```sh
set RUST_LOG=info       # found exe + family classification
set RUST_LOG=debug      # + final argument list
set RUST_LOG=trace      # + argument/name/expect internals, candidate search
```

---

## Runtime environment variables

| Variable | Effect |
|----------|--------|
| `WRAPPER_PREFER_VS` | Prefer Visual Studio's LLVM executables over ROCm LLVM when locating tools. |
| `WRAPPER_CLANG_CL_IS_LLVM` | Route `clang-cl` down the LLVM family (the default). Declaring it explicitly makes the intent immune to default changes. |
| `WRAPPER_CLANG_CL_IS_MSVC` | Force `clang-cl` to use the MSVC family instead of the default LLVM family. |
| `WRAPPER_SPLIT_FLAGS` | Split fused `/Fd<dir>` / `/Fo<dir>` flags (**off by default**; enable to turn the split step on). |
| `WRAPPER_SKIP_BAD_FLAGS` | Skip removing known-bad flags. |
| `WRAPPER_SKIP_SWAP_FLAGS` | Skip swapping problematic flags. |
| `WRAPPER_SKIP_ADD_FLAGS` | Skip adding extra helpful flags. |
| `WRAPPER_SKIP_ALL_FLAGS` | Disable the split / remove-bad / swap / add steps at once. |
| `WRAPPER_SKIP_VERSION_ON_EMPTY` | Skip auto-adding `--version` when no arguments remain. |
| `WRAPPER_ARGS_CHAR_LIMIT` | Override the response-file threshold (default `30000`). |
| `WRAPPER_FORCE_RESPONSE_FILES` | Always emit a response file, regardless of argument length. |
| `WRAPPER_ENABLE_PASSTHROUGH` | Bypass all processing and pass arguments through unchanged. |
| `WRAPPER_OPTIONS` / `WRAPPER_HELP` | Print the help banner and exit. |
| `RUST_LOG` | Set `env_logger` diagnostic verbosity (`error` default, up to `trace`). |

Each is a presence/flag variable: define it (to any value) to enable, except
`WRAPPER_ARGS_CHAR_LIMIT`, which takes a number. Note that split now requires
`WRAPPER_SPLIT_FLAGS` — it is the only step that is disabled unless explicitly
requested.
---

## Testing

```sh
cargo test          # runs the library unit tests (argument pipeline)
cargo test --lib    # targeted: only the src/lib.rs suite
```

The tests cover splitting of fused flags (slash- and dash-prefixed, mixed,
bare-passthrough), bad-flag removal, swap behaviour, the `skip_*` combinations,
extra-flag splicing (placement rules, duplicate suppression, link-step
guard), response-file emission (content, special characters, existing `@` arg),
and the empty-argument `--version` fallback.

---

## Notes / caveats

* The `sccache*` wrappers derive the real tool name from the wrapper name
  (`sccache-clang …`) or, for the bare `sccache` wrapper, from the first
  argument, and let `sccache` drive the actual tool.
* All raw flag tables (bad / swap / extra) are declared once per family/kind in
  `lib.rs`; classification is done by keywords on the deputy executable's name,
  so unrecognised tools default to `UNKNOWN` (no filter pack, and the `UNKNOWN`
  combination panics in `get_args_filter_pack`).
* Output found when searching the baked-in paths is reported at `info` level;
  enable `RUST_LOG=debug`/`trace` to see the full rewritten argument list.