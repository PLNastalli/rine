# nt-registry

Registry virtual por Capsule (memória em v0.1; arquivo em v0.2).

- Não é: hive do host, parser `NTUSER.DAT`.
- API: `Registry::{set,get,save,load}`, `RegValue`. Deps: `winabi`, `toml`.
  Consumers: Capsule (`registry.toml`; advapi32 guest em v0.3).
  Erros: `NtStatus`. Unsafe: nenhum.
- Testes: set/get + save/load roundtrip. Futuro: ACLs de chave.
