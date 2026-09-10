# bench/ — baselines de performance (medir antes de otimizar)

Método: examples determinísticos (`cargo run --example`), máquina do dev,
build `dev`. Thresholds tolerantes; compatibilidade > performance.

- `baselines/`: saídas promovidas (este arquivo = verdade atual).
- Comparar manualmente ao suspeitar de regressão absurda (>2x sem motivo).

## Como medir

```bash
cargo run -q -p pe --example bench_parse
cargo run -q -p runtime --example bench_load
```
