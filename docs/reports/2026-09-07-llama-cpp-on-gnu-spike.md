# Spike: can llama.cpp be linked into the engine crate on `stable-x86_64-pc-windows-gnu`?

**Date:** 2026-09-07. **Asked because** the Knowlu spec §5.3 chooses "llama.cpp through its Rust
binding" as the local inference runtime for plan 3, and the whole product is built with the GNU
toolchain, which is not that binding's mainstream path. Run before writing plan 3, in the same
spirit as plan 4a's Task 1 bundler spike. **Nothing was committed to the crate**; the probe lived in
a throwaway directory that has been deleted.

## Answer

**No, not today, without forking a dependency.** The obstacle is not ggml and not the bindings —
both work. It is a vendored HTTP library that llama.cpp builds unconditionally and that does not
compile under mingw-w64.

## What was tried, in order

| # | Failure | Cause | Resolution |
|---|---|---|---|
| 1 | `Unable to find libclang` | `llama-cpp-sys-2` 0.1.156 generates bindings with bindgen 0.72.1, which needs `libclang.dll`. None existed anywhere on the machine. | `winget install LLVM.LLVM` → `C:\Program Files\LLVM\bin` |
| 2 | `ggml.h:211: 'stdbool.h' file not found` | clang on Windows targets MSVC by default and does not pick up the mingw sysroot or, here, its own resource headers. | `BINDGEN_EXTRA_CLANG_ARGS=--target=x86_64-w64-mingw32 -isystem <llvm>\lib\clang\22\include -isystem <mingw>\x86_64-w64-mingw32\include -isystem <mingw>\include`. **Bindings then generated cleanly.** |
| 3 | cmake could not create `.../CMakeScratch/TryCompile-xxxx` | Windows `MAX_PATH`. The probe was running under a scratch directory whose prefix is already about 150 characters. Not a toolchain property. | Re-ran from `C:\Users\danie\ls` |
| 4 | `httplib.cpp:1520: '::CreateFile2' has not been declared` | llama.cpp vendors `cpp-httplib`, which calls a Windows 8+ API that this mingw-w64 (WinLibs POSIX **MSVCRT**, gcc 16.1.0) does not declare. | none found |
| 5 | identical | `CXXFLAGS=-D_WIN32_WINNT=0x0A00` did not reach the compile: `llama-cpp-sys-2`'s `build.rs` sets `CMAKE_CXX_FLAGS` explicitly, which overrides it. | none found |

**ggml itself is fine.** Attempt 5 compiled `ggml-cpu`, `quants.c`, `repack.cpp`, `sgemm.cpp` and
the rest to about 11% before the httplib target failed, so the numerical core builds under mingw.
The crate's `build.rs` already passes `LLAMA_BUILD_SERVER=OFF`, `LLAMA_BUILD_TOOLS=OFF`,
`LLAMA_BUILD_EXAMPLES=OFF` and `LLAMA_CURL=OFF`; `cpp-httplib` is built regardless of all four. So
the blocker is a component we do not want, built unconditionally, and closing it means vendoring or
patching the dependency.

## What linking would cost even if attempt 6 succeeded

Four prerequisites, none discoverable from its own failure message, every one of them required of
anyone who builds this repo, on top of mingw-w64:

1. LLVM installed, for `libclang.dll`.
2. Three `BINDGEN_EXTRA_CLANG_ARGS` include paths, correct for the installed clang version.
3. A build directory short enough for `MAX_PATH`.
4. A fork or patch of `llama-cpp-sys-2` to drop `cpp-httplib`.

It would also pull a large C++ build into the crate that the Python-versus-Rust dual-run harness
shares, and that has to stay quick to build and test at zero warnings.

## Recommendation

**Ship llama.cpp as a second Tauri sidecar rather than linking it**, and record it as a deviation
from spec §5.3. The reasons are independent of this spike's failure:

- The app already ships and tests a sidecar. `bundle.externalBin`, the placeholder in `build.rs`
  and the 1 MiB staging guard in `release.ps1` all exist and are proven by the 2026-09-07 release
  rehearsal.
- The C++ never enters our build, so none of the four prerequisites above reaches a contributor,
  and `cargo test` stays what it is.
- Upstream publishes prebuilt Windows binaries, so we consume a release artefact instead of
  maintaining a fork.
- The spec already downloads the **models** after install rather than bundling them, so extending
  that to the runtime keeps the installer at its current 4.76 MiB and preserves "the app runs with
  no model present" as the free-tier default.
- The constraint §5.3 actually cared about, grammar-constrained decoding on every call, is
  available at the process boundary.

**The costs to weigh:** a process lifecycle to manage; a protocol; a second thing to version; and,
if the runtime is downloaded rather than bundled, a downloaded executable that must be verified
before it is run — which the updater's minisign discipline already gives a shape for.

**This is a spec change and needs Quinn's word before plan 3 is written**, because §5.3 names the
binding specifically and decision 12 in §10 records it.
