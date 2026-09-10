# Arquitetura (REAL, v0.1)

> Este documento descreve o que EXISTE hoje, não o desejo. Divergência
> implementação↔doc é bug: corrija um dos dois imediatamente.

## Pipeline de execução

```text
rine [--capsule cap.toml] foo.exe args...
  │  Capsule::load_toml (drives, current_dir) + cmdline = argv.join(" ")
  ▼
Emulator::load(capsule, bytes, cmdline)
  │  pe::Image::parse → loader::map_image → nt_loader relocs/imports
  │  → protect_sections → track_external por seção (gerente do processo)
  │  → UTF-16 cmdline + params block (UNICODE_STRING em 0x70) → PEB/TEB + GS
  ▼
call entry_point()             (código x86_64 original, direto na CPU)
  │  IAT → kernel32!/ntdll! `extern "win64"` → kernelbase → ntdll → nt-*
  │  estado do processo via `ntdll::ProcessContext` (tabela, mem, fsys, cmdline)
```

## Camadas (dependência só para baixo)

```text
launcher (bine `rine`)
  ↓
runtime (Emulator, Capsule, resolvedor de imports)
  ↓      ┌──────────────────────────────────────────┐
  ├─────►│ façades: kernel32 → kernelbase → ntdll   │  finas; `extern "win64"`
  │      └──────────────────────────────────────────┘
  ├─────►│ subsistemas: nt-object/nt-memory/nt-file/... │ tipados, safe
  │      └──────────────────────────────────────────┘
  ├─────►│ imagem: pe + nt-loader + loader          │
  ▼      └──────────────────────────────────────────┘
host-linux (ÚNICA crate que chama libc/syscalls)
  ▼
kernel Linux
```

Diagrama ASCII da chamada guest→host (v0.1, verificado com gdb):

```text
guest .text (0x1400001000, RX)
  call [0x1400002090] ──IAT──► kernel32::WriteFile_impl (extern "win64")
                                   │ (RCX,RDX,R8,R9 + [RSP+0x28])
                                   ▼
                              kernelbase::write_file (safe)
                                   ▼
                              ntdll::nt_write_file
                                   ▼
                              nt_file::write_file → host_linux::write_all(fd 1)
```

## Onde mora cada decisão (para não colocar lógica no lugar errado)

- Parse de bytes → `pe`. Aritmética de RVA/reloc/import → `nt-loader`.
  mmap/mprotect → `loader` via `host-linux`. Semântica de objetos/handles →
  `nt-object`. Semântica de arquivos → `nt-file`. Marshalling Win32 →
  `kernelbase`; `kernel32` só converte ABI. Orquestração → `runtime`.
- `host-linux` é o único lugar que importa `libc`. Audite com
  `grep -rn "libc::" crates --include=*.rs | grep -v host-linux` (deve ser vazio
  exceto o próprio `host-linux`).

## Singleton v0.1–v0.2 (limitação consciente)

As exports `extern "win64"` têm assinatura fixa do Windows, logo não recebem
`&self`: leem o `ntdll::ProcessContext` global instalado por `Emulator::load`.
É o equivalente v0.1 do `GS:[PEB]` real. UM singleton de processo
(tabela + `MemoryManager` + `DriveMap` + cmdline), não vários.
Remoção planejada: contexto por-TEB via GS (ADR-0003). Um processo emulado
por processo host; `ExitProcess` = `std::process::exit` (ponto único).
`Capsule` (fonte, em `runtime`) vs `DriveMap` (artefato resolvido em
`nt-file`): conversão unidirecional no load, lógica de tradução num só lugar.

## O que NÃO existe ainda

GUI, DirectX, COM, .NET, threads extras, SEH/signals, WoW64, serviços,
`VirtualQuery`, advapi32 guest. Ver `roadmap.md`.
