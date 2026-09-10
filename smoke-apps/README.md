# smoke-apps — aplicações reais suportadas (sem binários no git)

Binários NÃO são commitados (`.gitignore`: `*.exe`). Cada app lista como
obtê-la/gerá-la e qual teste a cobre. `cargo test -p launcher` É a suíte
smoke (filho isolado por app). App nova estável → entrada aqui + teste.

| App | Origem | Teste | Esperado |
|---|---|---|---|
| hello.exe | `pe::builder::build_minimal_hello` (hand-built) | `launcher/tests/hello.rs` | `Hello World`, exit 0 |
| file/alloc/args.exe | `pe::builder` | `launcher/tests/v02.rs` | exits 11/66/16 |
| suite.exe | `pe::builder::build_suite_exe` | `launcher/tests/suite.rs` | exit 0 + marcadores + arquivo |
| evil.exe | `pe::builder::build_evil_exe` | `launcher/tests/suite.rs` | 12 recusas, exit 0 |
| hello_mingw.exe | `tests/windows/build_mingw.sh` (MinGW; requisitos v0.3) | `launcher/tests/mingw_demand.rs` (demanda) | falha limpa com 45 faltantes |

`hello_mingw.exe` executando de verdade é o critério de saída do v0.3.
