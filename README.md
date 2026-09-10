# Rine — runtime de compatibilidade Windows para Linux (Rust)

> Windows por fora, Rust seguro e modular por dentro, Linux nativo embaixo.

Executáveis Windows `.exe` x86_64 rodando no Linux como aplicativos quase-nativos:
código x86_64 original executa **diretamente na CPU** (sem emulação, sem Wine,
sem VM), com a semântica NT/Win32 preservada por um runtime modular em Rust
sobre primitivas Linux nativas.

```text
Windows PE x86_64
        ↓
código x86_64 executando diretamente na CPU
        ↓
Windows ABI / NT semantics (ntdll/kernelbase/kernel32)
        ↓
runtime modular em Rust (nt-memory, nt-object, ...)
        ↓
primitivas Linux nativas (mmap, sockets, futex, Vulkan, Wayland, PipeWire)
```

**Estado (v0.1, 2026-09-10):** `hello.exe` PE32+ real executa e imprime
`Hello World` via `GetStdHandle`/`WriteFile`/`ExitProcess` implementados sobre
Linux. Prova: `cargo test -p launcher --test hello`.

## Para continuar o trabalho (regra de continuidade para IA)

Leia nesta ordem, nunca a partir da memória:

1. `docs/current-state.md` — onde estamos, o que falta, próximo passo
2. `docs/architecture.md` — arquitetura REAL atual
3. `docs/roadmap.md` — milestones e critérios
4. `docs/dependency-map.md` — dependências entre crates
5. Documentação do subsistema a alterar (`docs/subsystems/*`) + ADRs relacionados

## Uso

```bash
cargo build --workspace
cargo run -p pe --example dump_hello -- /tmp/hello.exe
./target/debug/rine /tmp/hello.exe   # imprime: Hello World
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Estrutura

```text
crates/        # workspace Rust (19 crates; ver docs/dependency-map.md)
  winabi/      # tipos/layouts/constantes da ABI Windows (vocabulário comum)
  pe/          # parser PE32+ + builder de PEs de teste
  host-linux/  # ÚNICA fronteira com syscalls Linux
  nt-*/        # subsistemas internos tipados (object, memory, file, ...)
  nt-loader/   # lógica de imagem (relocs, imports)
  loader/      # mapeador (mmap + proteções)
  ntdll/ kernelbase/ kernel32/  # façades de DLL (finas)
  runtime/     # orquestração + execução do entry point
  launcher/    # binário `rine`
compat/        # profiles/ + quirks/ (workarounds documentados, nunca no núcleo)
tests/         # behavior/ integration/ differential/ windows/
docs/          # memória persistente do projeto (obrigatória)
```

## Princípios

- **APIs externas preservadas exatamente** (calling convention, layouts,
  NTSTATUS, handles, side effects). APIs internas modernas e tipadas.
- **Não copiar a arquitetura do Wine** (referência comportamental apenas).
- DLLs são **fachadas finas**; lógica vive nos subsistemas (`nt-*`).
- `unsafe` só nas fronteiras (ABI Windows / Linux / memória crua), cada bloco
  com justificativa `SAFETY`.
- Diferencial testing: Windows real é o oracle; todo bug ganha regression test.
- Documentação é requisito de primeira classe (ver `docs/`).

Mais: `docs/architecture.md`, `docs/adr/`, `docs/glossary.md`.
