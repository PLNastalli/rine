# tests/windows — fontes Windows + probe do oracle

- `hello_mingw.c`: hello CRT normal (`build_mingw.sh` compila; binário NÃO
  vai ao git). Demanda atual em `hello_mingw.imports.json` (46 imports,
  extraídos com o parser `pe`; para regenerar: `rine-api-scan scan` num dir
  com o exe e ler `imports` do JSON da DLL).
- `oracle_probe.c`: produtor Windows do formato `OracleResult` (3 casos
  espelhando `file/evil.exe`). Compila com MinGW aqui; RODA SÓ no Windows
  real. Saída: um JSON por linha (ver `crates/oracle`).
- `build_mingw.sh`: compila ambos para `$OUT` (default `/tmp/rine-win`).
