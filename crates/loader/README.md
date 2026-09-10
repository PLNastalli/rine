# loader

Mapeador PE: reserve → copy → protect. `MappedImage` com `Drop=munmap`.

- Não é: relocs/imports (delega a `nt-loader`), execução.
- API: `map_image`, `protect_sections`, `MappedImage::entry_point`.
  Deps: `pe`, `nt-loader`, `nt-memory`, `host-linux`. Consumers: `runtime`.
- Invariantes: ranges validados antes de copiar; RW até pós-imports.
  Unsafe: cópias com `copy_nonoverlapping` (região recém-mapeada).
- Testes: magic do entry, proteção. ADRs: 0001, 0005. Futuro: ASLR, DLL map.
