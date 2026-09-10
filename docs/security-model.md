# Modelo de segurança (threat model do Rine)

Executáveis Windows são **código não confiável**. Estado v0.2:

## O que executa onde

- Código guest x86_64: **mesmo processo e mesmo UID do host** (sem sandbox).
  Pode ler/escrever toda a memória do processo, chamar qualquer endereço
  mapeado e emitir syscalls diretamente (nada o impede — limitação v0.2).
- Runtime Rust: mesmo processo; `unsafe` só nas fronteiras auditáveis
  (`docs/safety-model.md`).
- Syscalls do runtime: só via `host-linux` (mmap, read/write, arch_prctl).

## Superfícies e limites atuais

| Superfície | Limite v0.2 |
|---|---|
| Filesystem | restrito a `DriveMap` (drives da Capsule + CWD); sem acesso fora do mapeado **pelo runtime** (guest com shellcode próprio não é contido) |
| Processos | `ExitProcess` = exit do host; sem spawn (`CreateProcess` inexistente) |
| Rede | sem sockets (nenhuma API) |
| Registry | virtual em memória/arquivo; nunca o do host (não há) |
| Env/host | guest herda env do host (documentado; filtrar em v0.4) |
| Devices/IPC | sem APIs |
| Capsule | declarativa (`capsule.toml`); permissões por-capsule em v0.4 |

## Ponteiros selvagens do guest

Sem SEH até v0.3: guest que dereferencia lixo derruba o host (SIGSEGV).
`evil.exe` cobre o que É contido (handles/flags/paths); o resto é fronteira
documentada, não promessa.

## Direção (v0.4+)

Isolamento por processo (fork/namespaces/seccomp), Capsule com permissões
(filesystem, rede, env filtrado), SEH contendo falhas do guest. Até lá:
rodar apenas PEs confiáveis/testes — como qualquer loader em desenvolvimento.
