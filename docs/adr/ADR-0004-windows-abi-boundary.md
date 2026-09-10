# ADR-0004 — Fronteira ABI: externa exata, interna tipada

- Status: aceito (2026-09-10).
- Context: como expor DLLs sem contaminar o interior com C-isms?
- Decision: camada de façade (`kernel32/kernelbase/ntdll`) com funções
  `extern "win64"`, nomes/símbolos/assinaturas Windows; interior 100% Rust
  (`NtResult`, newtypes, `Arc`, traits). Sem ABI Rust instável entre
  componentes futuros: fronteiras independentes usarão C ABI versionada/IPC.
- Alternatives: (a) tudo `extern "C"` cru até o núcleo (acoplamento, erros por
  convenção); (b) IDL próprio prematuro (custo sem benefício em v0.1).
- Why: compatibilidade observável na borda + refatoração livre dentro.
- Consequences: marshalling explícito nas façades (validado por testes E2E);
  `non_snake_case` permitido só nesses arquivos.
- Risks: divergência aridade/convenção (mitigado por PEs de teste + gdb).
