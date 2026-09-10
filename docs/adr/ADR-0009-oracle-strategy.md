# ADR-0009 — Estratégia oracle: api-db + formato versionado, sem host Windows

- Status: aceito (2026-09-10).
- Context: compatibilidade exige oracle (estrutural + comportamental), mas
  não há host Windows neste ambiente; a referência tem 4.4G (não vai ao git).
- Decision:
  1. `rine-api-scan` (reusa o parser `pe`) gera `api-db/` sharded com
     proveniência; só fixtures pequenas (KERNEL32/NTDLL) + resumos vão ao git.
  2. Cobertura com estados honestos (`Missing` default; `DifferentiallyVerified`
     exige medição — zero hoje por construção).
  3. Formato `OracleResult` schema 1 compartilhado; produtor Rine real
     (`oracle::run_case`) + template C compilável com MinGW (`probe`,
     executa SÓ no Windows). Leitores rejeitam schema desconhecido.
  4. Nenhum resultado Windows fictício em nenhum artefato (violação = bug).
- Alternatives: (a) commitar api-db cheia (122M — não); (b) esperar host
  Windows para começar (perde o valor estrutural imediato); (c) fixtures
  manuais de exports (deriva; geradas são melhores).
- Why: conhecimento versionado e regenerável desde o dia 1; demanda MinGW
  já é dirigida por dados reais (46 imports).
- Consequences: `serde/serde_json` no workspace (saída legível-por-máquina
  é o deliverable); 2 crates novas (`api-scan`, `oracle`).
- Risks: deriva fixture↔referência (mitigado: regenerar + metadata com build).
