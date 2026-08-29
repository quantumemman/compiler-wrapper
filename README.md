# WRAPPER — Compiler Argument Wrapper Helper

A dependency-light Rust wrapper for C/C++ toolchain executables (Clang/LLVM,
MSVC, GCC, GNU `ld`) and their `sccache` variants. Each wrapper:

1. **Locates** the real underlying toolchain executable from a baked-in list of
   install paths.
2. **Cleans / rewrites** command-line flags so arguments authored for one
   toolchain work on another (e.g. MSVC flags fed to LLVM Clang).
3. **Dispatches** to the target executable with the rewritten arguments,
   forwarding the exit status.

The wrappers are thin binaries that all share the same logic in `src/lib.rs`
(the `wrapper` library crate). Each binary's `main` is a few lines: initialize
the logger, optionally print usage help, then build a `Kind` and run it.

---

## Project layout

```
wrapper/
├── Cargo.toml        # crate manifest; defines every [[bin]] wrapper
├── src/
│   ├── lib.rs        # all shared logic (filtering, lookup, classification)
│   └── *.rs          # one near-identical main per wrapper binary
├── bin/              # (optional) deployed/copied wrapper executables
└── target/           # cargo build output (git-ignored)
```

### Binaries

| Wrapper (`cargo build` target) | Underlying tool it dispatches to |
|--------------------------------|----------------------------------|
| `cl-rs`        | MSVC C/C++ compiler (`cl`) |
| `clang-rs`     | Clang C compiler |
| `clangpp-rs`   | Clang++ C++ compiler |
| `clang-cl-rs`  | Clang in MSVC-compatible mode (`clang-cl`) |
| `gcc-rs`       | GNU GCC |
| `gpp-rs`       | GNU G++ |
| `hipcc-rs`     | HIP compile driver |
| `link-rs`      | MSVC linker (`link`) |
| `lld-link-rs`  | LLVM `lld-link` |
| `ld-rs`        | GNU `ld` |
| `sccache-rs`   | `sccache` (pass-through driver) |
| `sccache-clang-rs`, `sccache-clangpp-rs`, `sccache-clang-cl-rs` | `sccache` + Clang family |
| `sccache-gcc-rs`, `sccache-gpp-rs` | `sccache` + GCC/G++ |
| `sccache-hipcc-rs` | `sccache` + HIP |

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
| `HIP_PATH_`     | ROCm HIP install root |
| `LLVM_PATH_VS_` | Visual Studio’s bundled LLVM (Clang at `LLVM/../../VC/Tools/Llvm`) |
| `MSVC_PATH_`    | MSVC toolchain root |
| `LLVM_PATH_`    | ROCm / standalone LLVM install root |
| `GCC_PATH_`     | GCC install root |
| `PY_PATH_`      | Python scripts directory |

> These trailing-underscore names are read via the `env!()` macro, i.e. they are
> expanded when the crate is compiled, not at runtime.
---

## Runtime behaviour

Print the full usage/help banner (also lists every runtime variable):

```sh
set WRAPPER_HELP=1    # PowerShell: $env:WRAPPER_HELP="1"
clang-rs.exe          # prints the help box and exits
```

`WRAPPER_OPTIONS` has the same effect as `WRAPPER_HELP`.

### Argument pipeline

Unless a step is disabled (see variables below), each wrapper rewrites the
argument list through these stages:

1. **Split** fused MSVC flags — `/Fd<dir>` and `/Fo<dir>` expand into
   `/Fd` + `<dir>` / `/Fo` + `<dir>`.
2. **Remove bad flags** — drop flags that are known to be invalid for the target
   toolchain (e.g. `/EHsc`, `/bigobj`, `/permissive-`, MSVC-only `/W3` etc.).
3. **Swap problematic flags** — regex-based rewrites, e.g. MSVC `/Zi` → `-g`,
   `/MD`/`/MDd` → `-fms-extensions`, linker `/LTCG` → `-flto`, and `/01`–`/04`
   → the matching `-01`–`-04` (as written in the flag tables).
4. **Add helpful flags** — splice in extra flags after a `-x` argument (avoiding
   duplicates) that improve cross-toolchain compatibility.
5. **Version fallback** — if the argument list ends up empty, a lone `--version`
   is appended so the tool still prints something useful.
6. **Response files** — if the joined arguments exceed `WRAPPER_ARGS_CHAR_LIMIT`
   (default `30000`) or `WRAPPER_FORCE_RESPONSE_FILES` is set, the arguments are
   written to a `@*.rsp` file and the list collapses to a single `@file` argument.

The exact bad/swap/extra flags are chosen by classifying the target executable
from its name against keyword lists:

* **Family** — `LLVM` (`clang|llvm|lld`, but `clang-cl` ⇒ MSVC), `MSVC`
  (`cl|link`), `GCC` (`gcc|g++|ld`).
* **Kind** — `COMPILER` (`clang|hip|cl|gcc|g++`) or `LINKER` (`link|lld|ld`).

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
| `WRAPPER_PREFER_VS` | Prefer Visual Studio’s LLVM executables over ROCm LLVM when locating tools. |
| `WRAPPER_SKIP_ALL_FLAGS` | Disable all of split / remove-bad / swap / add steps at once. |
| `WRAPPER_SKIP_SPLIT_FLAGS` | Skip splitting fused `/Fd<dir>` / `/Fo<dir>` flags. |
| `WRAPPER_SKIP_BAD_FLAGS` | Skip removing known-bad flags. |
| `WRAPPER_SKIP_SWAP_FLAGS` | Skip swapping problematic flags. |
| `WRAPPER_SKIP_ADD_FLAGS` | Skip adding extra helpful flags. |
| `WRAPPER_SKIP_VERSION_ON_EMPTY` | Skip auto-adding `--version` when no arguments remain. |
| `WRAPPER_ARGS_CHAR_LIMIT` | Override the response-file threshold (default `30000`). |
| `WRAPPER_FORCE_RESPONSE_FILES` | Always emit a response file, regardless of argument length. |
| `WRAPPER_ENABLE_PASSTHROUGH` | Bypass all processing and pass arguments through unchanged. |
| `WRAPPER_OPTIONS` / `WRAPPER_HELP` | Print the help banner and exit. |
| `RUST_LOG` | Set `env_logger` diagnostic verbosity (`error` default, up to `trace`). |

Each is a presence/flag variable: define it (to any value) to enable, except
`WRAPPER_ARGS_CHAR_LIMIT`, which takes a number.

---

## Testing

```sh
cargo test          # runs the library unit tests (argument pipeline)
cargo test --lib    # targeted: only the src/lib.rs suite
```

The tests cover splitting of fused flags, bad-flag removal, swap behaviour,
the `skip_*` combinations, extra-flag splicing, response-file emission, and the
empty-argument `--version` fallback.

---

## Notes / caveats

* The wrapper is Windows-oriented: tool names are searched with an `.exe`
  extension and paths are normalised to `/`.
* `sccache*` wrappers derive the real tool name from the first argument (e.g.
  `sccache clang ...`) and let `sccache` drive the actual tool.
* GCC family currently defines no bad flags; the GCC pipeline is geared toward
  the swap / extra-flag steps.
* Output found when searching the baked-in paths is reported at `info` level;
  enable `RUST_LOG=debug`/`trace` to see the full rewritten argument list.