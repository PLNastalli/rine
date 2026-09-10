# Sidecar `rine` (o CLI dentro do Manager)

O instalador do Rine Manager entrega **dois binários lado a lado**:

```text
rine            ← o runtime/CLI (launcher)
rine-manager    ← a GUI (este crate)
```

O Manager localiza o runtime pelo sidecar (ao lado do executável atual),
antes de qualquer outro candidato (ver `manager-core::runtime` e
`docs/manager/runtime-interface.md`). Por isso a janela nunca precisa de
configuração no layout oficial — o erro `RUNTIME_NOT_FOUND` só aparece
quando o `rine` não foi instalado/empacotado junto.

## Como empacotar

```bash
crates/manager-tauri/pack-sidecar.sh   # release do launcher → binaries/
```

Isso gera `binaries/rine-<target-triple>` (ex.:
`rine-x86_64-unknown-linux-gnu`), que o `externalBin` do
`tauri.conf.json` instala ao lado do `rine-manager`.

## Dev (sem instalador)

```bash
cargo build -p launcher                # gera target/debug/rine
cargo run -p manager-tauri             # acha o sidecar em target/debug/
```

Os binários empacotados (`rine-*`) NÃO vão ao git (só este README).
