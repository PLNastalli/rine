<div align="center">

# Rine

### Windows runs further.

**A Rust-native Windows compatibility runtime for Linux.**

Run Windows x86_64 executables directly on the CPU —  
**no Wine, no Proton, no virtual machine, no CPU emulation.**

![Version](https://img.shields.io/badge/version-0.2.0--alpha.2-blue)
![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange)
![Platform](https://img.shields.io/badge/host-Linux-lightgrey)
![Guest](https://img.shields.io/badge/guest-Windows%20x86__64-0078D4)
![Status](https://img.shields.io/badge/status-early%20alpha-yellow)

</div>

---

## What is Rine?

Rine is an experimental Windows compatibility runtime written in Rust.

It loads Windows PE32+ x86_64 executables, maps them into memory, resolves
their Windows imports, and executes the original machine code directly on
the host CPU.

Windows APIs are then implemented by Rine and translated into native Linux
primitives.

```text
Windows PE x86_64
        │
        ▼
Original x86_64 code
executing directly on the CPU
        │
        ▼
Windows ABI / NT / Win32
        │
        ▼
kernel32 → kernelbase → ntdll
        │
        ▼
nt-memory / nt-file / nt-object / ...
        │
        ▼
host-linux
        │
        ▼
Linux kernel
```

Rine is **not an emulator**.

Rine is **not a virtual machine**.

Rine does **not use Wine or Proton internally**.

It is an independent compatibility runtime.

---

## Current status

> **Rine is early alpha software.**
>
> It is not yet intended to run arbitrary Windows applications.

Current version:

```text
0.2.0-alpha.2
```

Already working end-to-end:

- PE32+ x86_64 parsing
- native x86_64 guest execution
- image mapping
- relocations
- import/IAT resolution
- Windows x64 ABI calls
- section memory protections
- basic PEB/TEB environment
- command-line propagation
- virtual Windows drives
- file I/O
- Windows handles
- virtual memory
- per-application Capsules
- structured crash reporting
- API scanning
- differential-testing infrastructure
- deterministic fuzzing
- regression corpus
- performance regression gate

Current test executables:

```text
hello.exe
file.exe
alloc.exe
args.exe
suite.exe
evil.exe
```

They execute through Rine without Wine, Proton or a VM.

---

## Windows APIs implemented

### KERNEL32.dll

```text
GetStdHandle
WriteFile
ReadFile
CreateFileA
CloseHandle
VirtualAlloc
VirtualFree
VirtualProtect
GetCommandLineW
TlsAlloc
TlsFree
TlsGetValue
TlsSetValue
GetLastError
Sleep
ExitProcess
```

### NTDLL.dll

```text
RtlExitUserProcess
NtTerminateProcess
```

Additional NT functionality such as `NtWriteFile` already exists internally
inside the runtime.

Rine deliberately does **not** count fake-success stubs as compatibility.

If an import is unsupported, the loader fails explicitly instead of silently
pretending that the API works.

---

## The first proof

A Windows PE executable can already execute:

```bash
./target/debug/rine hello.exe
```

Output:

```text
Hello World
```

The call path is real:

```text
guest .exe
   │
   ▼
IAT
   │
   ▼
kernel32::WriteFile
   │
   ▼
kernelbase
   │
   ▼
ntdll
   │
   ▼
nt-file
   │
   ▼
host-linux
   │
   ▼
Linux write()
```

The original Windows x86_64 instructions execute directly on the processor.

---

## Why Rust?

A compatibility runtime sits directly on a hostile ABI boundary.

Windows programs can pass:

- raw pointers
- handles
- malformed PE structures
- invalid paths
- incorrect sizes
- unexpected thread state
- corrupted inputs

Rine therefore uses Rust for the runtime architecture while isolating
`unsafe` code to places where it is actually required:

```text
Windows ABI
raw guest memory
assembly / CPU state
Linux syscall boundary
```

The internal subsystems are designed to remain typed and safe whenever
possible.

---

## Architecture

Rine separates Windows-facing DLL façades from the actual operating-system
semantics.

```text
launcher
   │
   ▼
runtime
   │
   ├──────────────► loader ──► nt-loader ──► pe
   │
   ├──────────────► kernel32
   │                    │
   │                    ▼
   │                kernelbase
   │                    │
   │                    ▼
   │                  ntdll
   │
   └──────────────► nt-* subsystems
                        │
                        ▼
                    host-linux
                        │
                        ▼
                    Linux kernel
```

Examples:

```text
CreateFileA
    ↓
kernel32
    ↓
kernelbase
    ↓
nt-file
    ↓
openat/openat2
```

```text
VirtualAlloc
    ↓
kernel32
    ↓
kernelbase
    ↓
ntdll
    ↓
nt-memory
    ↓
mmap
```

`host-linux` is the only workspace layer allowed to directly interact with
Linux syscalls/libc.

This keeps Windows semantics separate from host implementation details.

---

## Workspace

Rine currently consists of more than twenty Rust crates.

```text
crates/
├── winabi
├── pe
├── host-linux
│
├── nt-object
├── nt-memory
├── nt-process
├── nt-thread
├── nt-file
├── nt-sync
├── nt-exception
├── nt-registry
├── nt-security
├── nt-loader
│
├── loader
│
├── ntdll
├── kernelbase
├── kernel32
│
├── runtime
├── launcher
│
├── api-scan
├── oracle
├── difftest
└── perf
```

The rule is simple:

> Windows on the outside.  
> Safe and modular Rust inside.  
> Native Linux underneath.

---

## Compatibility methodology

Rine does not try to implement Windows APIs randomly.

Compatibility development is demand-driven:

```text
real application
      ↓
missing import
      ↓
API database
      ↓
behavior test
      ↓
implementation
      ↓
validation
      ↓
permanent regression test
```

This prevents thousands of meaningless stubs from being mistaken for actual
compatibility.

An API can move through states such as:

```text
Missing
Stub
Partial
Implemented
BehaviorTested
DifferentiallyVerified
```

Compatibility claims require evidence.

---

## Windows API scanner

Rine includes `rine-api-scan`, which analyzes Windows PE modules and builds a
machine-readable view of the Windows API surface.

It extracts information including:

```text
DLL architecture
PE metadata
sections
imports
exports
ordinals
forwarded exports
API Sets
DLL dependencies
```

A Windows 11 25H2 System32 reference is used locally as an **oracle and
metadata reference**, never as a runtime dependency.

The scanner has already processed thousands of Windows modules and roughly
183,000 exports.

---

## Differential testing

Correctness matters more than merely having an export with the right name.

Rine therefore includes a differential-testing platform designed to execute
the same scenario against:

```text
Windows reference
        vs
Rine
```

and compare observable behavior.

The test infrastructure supports:

- deterministic seeds
- scenario generation
- normalization
- result comparison
- shrinking/minimization
- property-based testing
- parallel campaigns
- permanent regression corpora

It has already discovered real bugs in filesystem/path behavior that were
fixed and converted into regression tests.

---

## Fuzzing

The project also fuzzes critical compatibility boundaries.

Current infrastructure covers areas such as:

```text
PE parsing
memory
handles
filesystem
malformed guest input
```

Current development runs reach approximately:

```text
filesystem       ~3,000 scenarios/s
handles/memory   ~90,000–220,000 scenarios/s
PE fuzzing       ~1,000,000 cases/s
```

The objective is simple:

> Invalid Windows input must produce a controlled failure, not undefined
> behavior or host corruption.

---

## Performance

Rine tracks performance from the beginning rather than attempting to recover
it after the architecture becomes large.

Baseline `v0.2.0-alpha.1` (`bench/baselines/`, median of warmed-up batches):

```text
pe.parse.hello    2.023 µs/op
pe.parse.suite    4.524 µs/op
loader.full.suite 15.950 µs/op
```

The loader benchmark includes approximately:

```text
map image
+ relocations
+ imports
+ section protections
```

without entering the guest executable.

A performance regression gate (`rine-bench check`, warn-only in alpha:
>10% WARNING, >15% REGRESSION) keeps slowdowns visible without blocking
on shared-runner noise.

Performance is important, but correctness comes first.

> Never sacrifice Windows behavior just to win a benchmark.

---

## Capsules

Rine is being designed around per-application environments called
**Capsules**.

A Capsule can contain application-specific runtime state such as:

```text
virtual drives
filesystem mapping
current directory
registry
compatibility configuration
quirks
permissions
```

Compatibility hacks belong in explicit profiles and quirks rather than being
hidden inside the core runtime.

---

## Quick start

### Requirements

- Linux x86_64
- Rust 1.80+
- Cargo

Clone:

```bash
git clone https://github.com/PLNastalli/rine.git
cd rine
```

Build:

```bash
cargo build --workspace
```

Run a supported PE:

```bash
./target/debug/rine path/to/program.exe
```

Run the test suite:

```bash
cargo test --workspace
```

Development gates:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Roadmap

### v0.1 — Native PE execution ✅

```text
PE32+ loader
native x86_64 execution
basic kernel32 / ntdll
Hello World
```

### v0.2 — Process + filesystem ◀ current

```text
files
handles
virtual memory
Capsules
API database
scanner
fuzzing
differential infrastructure
performance gate
```

### v0.3 — CRT / threads / synchronization / exceptions

Primary target:

```text
hello_mingw.exe
```

Done so far:

```text
TLS (TlsAlloc/TlsFree/TlsGetValue/TlsSetValue)
GetLastError
Sleep
```

Major work remaining includes:

```text
critical sections
VirtualQuery
LoadLibrary
GetProcAddress
API Sets
initial SEH
```

### v0.4 — Isolation + dynamic loader

```text
process isolation
dynamic DLL loading
forwarders
TLS callbacks
ASLR
exception metadata
```

### Later

```text
User32
Wayland
GDI
Winsock
COM
PipeWire
DXGI / Direct3D → Vulkan
WoW64
services
.NET interoperability
```

Large subsystems are added only when the lower layers are proven stable.

---

## What Rine is not

Rine is not:

- a Windows virtual machine
- a CPU emulator
- a Wine fork
- a Proton fork
- a collection of success-returning API stubs
- currently a replacement for Wine

The long-term goal is broad Windows application compatibility through a
modern Rust architecture.

The current project is still an experimental alpha.

---

## Engineering principles

### Never silently break something that worked

Every discovered compatibility bug should become a permanent regression test.

### Windows ABI is sacred

Calling conventions, layouts, handles, error behavior and observable
semantics must match Windows where applications depend on them.

### No fake compatibility

An unsupported API should fail honestly rather than return bogus success.

### Linux stays below the compatibility boundary

Windows semantics live in Rine's NT subsystems.

Linux-specific operations stay in `host-linux`.

### Git keeps the past; the source tree represents the present

Dead implementations are removed after replacement and validation instead
of being accumulated as `_old`, `_v2`, backups or commented-out code.

---

## Documentation

Start here:

```text
AGENTS.md
docs/current-state.md
docs/architecture.md
docs/roadmap.md
docs/dependency-map.md
docs/subsystem-map.md
docs/safety-model.md
docs/security-model.md
docs/non-regression-policy.md
docs/cleanup-policy.md
docs/adr/
```

For AI agents working on Rine, `AGENTS.md` defines the project's engineering
rules and mandatory reading order.

---

## Legal / clean-room policy

Rine is an independent implementation.

Windows binaries and publicly observable Windows behavior may be used for
testing and interoperability research.

Proprietary Microsoft implementation code must not be copied into Rine.

The Windows reference tree used during development is an oracle, not a
dependency and is not distributed with the project.

---

## License

Rine is dual-licensed under:

```text
MIT OR Apache-2.0
```

---

<div align="center">

### Rine

**Windows runs further.**

*A Windows compatibility runtime for Linux.*

**Por um Linux melhor.**

</div>
