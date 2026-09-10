# PE loader

Vida de um executável (implementado em v0.1):

```text
disco → parse → reserve address space → map sections → relocs → imports
      → TLS (lido; callbacks v0.4) → PEB/TEB → entry point
```

## Detalhes v0.1

- **Parse** (`pe::Image`): DOS→`e_lfanew`→`PE\0\0`→COFF(AMD64)→Optional
  PE32+→16 DataDirs→seções. Erros tipados (`PeError`), sem panics.
- **RVA→offset**: headers pelo prefixo; resto por seção
  (`VirtualAddress ≤ rva < VA+max(VirtualSize,RawSize)`).
- **Imports**: `OriginalFirstThunk || FirstThunk`; ordinais
  (`0x8000...`) e Hint/Name; IAT RVAs coletados para patch.
- **Exports**: nomes+ordinais+forwarders (leitura; resolução de DLL própria
  em v0.4).
- **Relocs**: blocos `ADDR64` aplicados com `delta = base - preferida`;
  `REL32`/`ADDR32NB` validados como no-op; `ABSOLUTE` = padding.
- **Map** (`loader`): tenta `MAP_FIXED_NOREPLACE` na preferida, fallback
  anônimo; copia headers+raw, zero-fill `.bss`; valida ranges.
- **Protect**: `.text`→RX, dados→R/RW — APÓS relocs/imports (que exigem RW).
- **Entry**: `transmute` para `extern "win64" fn()`; RSP%16==8 por construção
  da CALL; GS=TEB via `arch_prctl`.

## Builders de teste (`pe::builder`)

Um emissor de headers (`assemble()`), `.rdata` genérico
(`build_rdata_generic`: blobs + Hint/Names + ILT/IAT + dir) e helpers
(`emit_call`, `emit_lea_rip`, `patch_rel8`, `emit_sub_rsp`, `emit_stack_arg`).
Regras de ouro documentadas no código: imm8 é com sinal (usar `emit_sub_rsp`);
args 1–4 em registradores, stack do 5º em diante (`emit_stack_arg`).
`hello.exe` congelado (hash verificado); `file/alloc/args.exe` no genérico.

## Decisões de teste que viraram regra

`written` (out-param) é local de stack, nunca global em `.rdata` — `.rdata`
é R-only após protect (SIGSEGV real encontrado via gdb, 2026-09-10).
