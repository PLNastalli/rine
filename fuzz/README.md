# fuzz/ — estratégia de fuzzing

Sem `cargo-fuzz`/libfuzzer (exige nightly; o projeto é stable por
reprodutibilidade). Em vez disso, **corpus determinístico versionado**:

- `crates/pe/tests/fuzz_corpus.rs`: 900 mutações (seeds fixas) dos PEs de
  teste + lixo puro. Propriedade: parse/views nunca panicam (`Ok`/`Err`).
- `crates/nt-file/tests/robust.rs`: paths NT hostis + registry/TOML hostil.

Para reproduzir/estender: seeds no código, mutações em `mutate()`.
Upgrade path: trocar o driver por libfuzzer mantendo as propriedades
(ver `docs/testing-strategy.md`). Corpus proprietário: nunca commitar
binários — gerar a partir dos builders.
