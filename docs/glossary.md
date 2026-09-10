# Glossário (vocabulário único — não renomear por módulo)

| Termo | Significado |
|---|---|
| PE / PE32+ | Formato executável Windows; 32+ = 64 bits. O arquivo continua PE (sem ELF). |
| RVA | Endereço relativo à ImageBase. Nunca confundir com VA ou offset de arquivo. |
| IAT / ILT | Tabela de endereços de import / tabela de lookup (nomes). Loader patcha a IAT. |
| PEB / TEB | Bloco de ambiente do processo / da thread; observáveis via GS no x86_64. |
| NTSTATUS | Código de 32 bits do NT (`0`=sucesso, bit31=erro). Domínio ≠ Win32. |
| Win32Error | `GetLastError`. Conversão NT→Win32 explícita. |
| NT Object | Objeto kernel tipado com refcount, nome opcional, rights. |
| Handle | ID geracional (`(gen<<32)\|idx`) na `HandleTable`. NÃO é fd. |
| Section | Objeto de memória compartilhável (futuro); ≠ seção PE. |
| Capsule | Config declarativa por aplicação (registry, drives, quirks, versão). |
| Quirk | Workaround app-específico documentado em `compat/quirks/` (nunca no núcleo). |
| Host Backend | `host-linux`: única fronteira com syscalls. |
| Windows Oracle | Referência (estrutural: api-db; comportamental: runner Windows). |
| Differential Test | Mesmo teste no Windows e no Rine + diff normalizado. |
| API Set | `api-ms-win-*`: DLL virtual (sem arquivo; resolve via schema). |
| Forwarder | Export que aponta para `OutraDLL.Simbolo` (first-class). |
| Matrix | `compatibility/matrix.json`: teste × build × Rine → status. |
| NT Layer | `ntdll` + `nt-*`: semântica NT interna tipada. |
| Win32 Layer | `kernelbase`/`kernel32`: façades finas com ABI Windows. |
| SEH | Exception handling estruturado Windows (`__try/__except`; v0.3). |
| WoW64 | Subsistema 32-bits sobre kernel 64 (futuro distante). |
