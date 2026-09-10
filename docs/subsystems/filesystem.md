# Filesystem

Mapeamento-alvo: `NtCreateFile → openat2/read/write` com a MENOR transformação
que preserve semântica. v0.2 implementado: `DriveMap::translate`
(`C:\`→drive da Capsule, `\??\`→strip, relativo→CWD; UNC/`\\.\`/`X:rel`→
`NOT_IMPLEMENTED` testado) + `create/read/write/close` com dispositions 1–5,
`owns_fd` (console nunca fecha), short-count em EOF. Abertura via `std::fs`
(openat/openat2 no kernel); `openat2` explícito com dirfd/flags em v0.3
(ADR-0007). Lacunas documentadas: share/flags ignorados, `access==0`→read-only.
Testes-guia: `file.exe` (E2E, exit 11) + unitários de tradução/roundtrip.
