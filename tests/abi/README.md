# tests/abi — suíte ABI (implementação: `crates/winabi/tests/abi.rs`)

Contrato binário Windows: `sizeof`/`alignof`/offsets de TIB/TEB/PEB,
valores de flags/NTSTATUS, e chamada `win64` de 6 args nos dois sentidos.
Falhou aqui = quebrou compatibilidade. Status: verde.
