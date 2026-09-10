# compatibility/ — matriz Windows × Rine

- `matrix.json`: verdade tabular (`test × windows_build × rine → status`).
  `RINE_PASS` = passa no Rine (medido); `PASS` pleno exige `windows_build`
  preenchido + diff limpo (regra diferencial). Coluna Windows hoje: `null`
  + `note` — oracle pendente, nunca fictício.
- `builds/`: uma pasta por build Windows medido (ex.: `win11-25h2/` com
  oracles `*.oracle.json` no formato `oracle` schema 1).
- `baselines/`: saídas de referência promovidas (só via teste verde).
- `regressions/`: diffs abertos (teste × esperado), um arquivo por caso,
  com issue/condição de fechamento.
