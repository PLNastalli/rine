# 2026-09-10 — Ciclo de infraestrutura/oracle (quinta sessão)

- Auditoria (missão): System32 em `windows-reference/win11-25h2`
  (3410 DLLs topo, 4.4G); MinGW 16.2 funcional; sem git (gap registrado).
- AGENTS.md criado (constituição). `hello_min.exe/Hello Rine!` da missão ≙
  nosso `hello.exe/Hello World` (mesma prova, bytes próprios).
- Parser: ordinais reais + forwarders estruturados + `machine`
  (testado contra kernel32 real; 2 suposições corrigidas pelo oracle).
- `rine-api-scan`: 5578 DLLs/183k exports/9k forwarders em ~3s;
  `api-db/` + cobertura honesta (12/4209 BehaviorTested).
- Oracle schema 1 (runner Rine + probe C compilável) + demanda MinGW
  (45 faltantes; fixture; `RINE-DEMAND` no launcher).
- ABI suite, fuzz determinístico, `RUST_LOG`, `RINE-CRASH`, matriz,
  benches+baseline, security-model. ADR-0009.
- KernelBase.dll existe (caixa alta — scanner normaliza).
- Gate verde; hello âncora intacto. Próximo: v0.3 pela demanda MinGW.
