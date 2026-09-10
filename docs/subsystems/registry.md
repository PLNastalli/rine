# Registry

Registry virtual por Capsule (nunca o registry do host). Árvore em memória
(`HKLM`/`HKCU`) = overlay; `registry.toml` = persistência (formato congelado
v0.2: `[\"HKLM\\...\"]` + `{s,dword,qword,bin-hex}`; `save`/`load` + teste).
APIs guest (`advapi32`) em v0.3.
