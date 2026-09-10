# Modelo de segurança de memória (safety-model)

## Fronteiras `unsafe` (únicas permitidas sem ADR novo)

```text
Windows ABI  ══════════  kernel32 (from_raw_parts, *lp_written)
safe Rust    ══════════  subsistemas nt-*
Linux/memória══════════  host-linux (mmap/mprotect/munmap/arch_prctl/write)
                         loader (copy_nonoverlapping para imagem RW)
                         nt-loader (escritas IAT/reloc com guard de range)
                         runtime (transmute entry + call, GS set/restore)
```

## Regras

1. Todo `unsafe` não-trivial tem comentário `SAFETY` com: invariantes, quem
   garante, duração, consequência de violação. (Verificar com
   `grep -rn "SAFETY" crates`.)
2. Nunca `unsafe` por conveniência. Alternativa safe sempre preferida.
3. `unwrap`/`expect` proibidos em caminhos normais do runtime (só testes e
   `main` de ferramentas; `Mutex::lock().unwrap()` tolerado com comentário —
   envenenamento = bug interno, aborta em vez de corromper guest).
4. Guest selvagem (ponteiro inválido) deve, no limite, matar o processo
   emulado (v0.1 = host, documentado), nunca corromper o host silenciosamente.
   Endurecimento progressivo: `VirtualQuery` guards (v0.2), SEH/signals (v0.3),
   isolamento por processo (v0.4).

## Propriedades por fronteira

- **mmap/munmap**: `nt-memory` rastreia regiões (`BTreeMap`); `release`
  exatamente uma vez; double-free = `Munmap` erro do kernel, não UB.
- **IAT/relocs**: cada escrita checa `rva + 8 <= size_of_image`; fora → `Err`,
  nunca escrita selvagem.
- **GS/TEB**: `Box<TebMinimal>` vive em `Emulator`; GS restaurado no retorno
  normal; `ExitProcess` não restaura porque o processo termina.
- **Entry call**: `transmute` para `extern "win64" fn()`; sem args; alinhamento
  RSP%16==8 herdado da CALL Rust (verificado). Guest com stack própria
  corrompida = SIGSEGV contido.
- **Slices de guest** (`WriteFile_impl`): `from_raw_parts` válido pela duração
  da chamada por contrato Windows; mesma address space, sem retenção.

## Auditoria

```bash
grep -rn "unsafe" crates --include=*.rs | grep -v "SAFETY\|unsafe impl Send" | head
cargo geiger  # (futuro) contagem de unsafe por crate
```
