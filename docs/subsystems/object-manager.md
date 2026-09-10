# Object Manager

Tipos (`File/Event/Mutex/Semaphore/Process/Thread/Section/Timer/RegistryKey/
Token`), lifetime por `Arc`+`HandleTable`, nomes opcionais (`Namespace`),
rights (`u32` estilo `GENERIC_*`, enforcement em v0.5). Type-safety: `lookup`
+ checagem de `ObjectType` (mismatch → `INVALID_HANDLE`, nunca cast cego).
Resolução de nomes hierárquica e symlinks `\??\` em v0.2–v0.3.
