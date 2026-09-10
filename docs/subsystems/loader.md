# Loader (mapeador)

Responsabilidade: adquirir memória do host e materializar bytes do PE.
Não interpreta semântica (isso é `nt-loader`); não executa (isso é `runtime`).

Passos (`loader::map_image`): validar `SizeOfImage` → reservar (preferencial
com `MAP_FIXED_NOREPLACE`, fallback anônimo — nunca destrói mapeamento alheio)
→ copiar headers → copiar raws com checagem de range → zero-fill além de
`RawSize` até `VirtualSize` → validar entry. `protect_sections` depois.
`MappedImage::Drop` faz `munmap` (RAII; vazamento impossível por esquecimento).

Testes: `loader` unit (magic bytes do entry, proteção aplicada).
