# api-db — base de conhecimento da referência Windows (GERADA)

Oracle estrutural, nunca dependência do runtime. Fonte:
`windows-reference/win11-25h2/System32` (4.4G, fora do git).

- **Fonte**: `windows-reference/...` (build Win11 25H2; ver `metadata.json`)
- **Generator**: `rine-api-scan` (`crates/api-scan`), versão no `metadata.json`
- **Versão do formato**: `schema: 1` em cada arquivo
- **Como regenerar**: `cargo run -p api-scan -- scan <System32> api-db/<build>/`
  + `cargo run -p api-scan -- coverage api-db/<build>/`
- **NUNCA editar à mão** (regenerar; o cabeçalho `provenance` marca).

Layout (`windows-11-25h2-x64/`):

```text
metadata.json      # contagens + proveniência
coverage.json      # kernel32+ntdll × Rine (status honestos)
apisets.json       # arquivos api-ms-win-*/ext-ms-win-* (virtuais; sem DLL física)
dlls/KERNEL32.json # fixture: exports reais p/ cobertura determinística
dlls/NTDLL.json    # idem
dlls/*.json        # GERADO LOCAL (122M p/ System32 cheia) — NÃO commitar
forwarders.json    # GERADO LOCAL (1.8M) — regenerar, não commitar
```

Estados de cobertura (`ApiStatus`): Missing / Stub / Partial / Implemented /
BehaviorTested / DifferentiallyVerified — definições em
`docs/compatibility.md`. Hoje: 12 BehaviorTested, 4197 Missing, resto zero.
Nada é "implementado" sem teste; nada é "diferencial" sem host Windows.

`.gitignore` local ignora o gerado volumoso (quando houver repo git).
