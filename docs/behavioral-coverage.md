# Cobertura comportamental

Code coverage não basta: medimos DIMENSÕES exercitadas por API.

## Dimensões declaradas (`difftest::gen`)

`rine-test coverage <alvo>` lista o espaço mensurável:

- `CreateFileA::path_class`: missing, existing, relative, nested,
  drive-missing, unc, trailing, unicode
- `CreateFileA::access`: read, write, rw, zero
- `CreateFileA::disposition`: new, always, existing, open-always, truncate, bad
- `fileops::op`: create, write, read, close (+ handles/memory/pe equivalents)

## Medição

Toda campanha registra `(dimensão, valor)` vistos em `report.json → coverage`.
Exemplo de leitura: se `disposition=truncate` tem 0 ocorrências, a dimensão
está cega — priorize gerador antes de aumentar quantidade
(1M de casos ruins < 100 bons).

## Priorização automática

`RINE-DEMAND` (imports de apps reais) + divergências alimentam prioridade;
`--merge-matrix` agrega contagens em `compatibility/matrix.json`
(`campaigns[]`, nunca apagando entradas). Divergência em app real >
divergência sintética.
