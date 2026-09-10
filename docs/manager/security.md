# Rine Manager — segurança (EXEs são não-confiáveis)

Herda `docs/security-model.md`: guest roda no mesmo UID, sem sandbox até
v0.4. A GUI não piora isso — regras:

1. **Nunca executa ao dropar**: drop/select só inspeciona (parse PE).
   Execução exige clique em Run (intencional, sempre).
2. **Sem `shell=true`, sem string de comando**: `run` usa argv estruturado
   (`Command` + args separados). Args do usuário nunca viram shell.
3. **Paths canonicalizados** quando apropriado; `..` não escapa o registro
   (exe deve existir e ser arquivo).
4. **Drives perigosos** (`/`, `$HOME`, `..` fora da capsule) exigem warning
   + confirmação explícita; default sugere subdir isolado
   (`~/Rine/capsules/<nome>/drive_c`).
5. **Sem root**: Manager roda como usuário; nada pede `sudo`. Operação
   futura com privilégio = isolada + explicada (nunca a GUI inteira).
6. **Sem rede automática**: nada em `manager-core` faz I/O de rede
   (Oracle futuro = opt-in explícito, developer mode).
7. **Erro nunca vago**: `ManagerError` → `{human, code, details?}` para o
   React; `unwrap/expect` proibidos em caminhos de input (regra do repo).
8. **Timeout mata o filho**: run pendurado não trava a UI nem vaza processo.
9. **Ícone de EXE**: só via parser próprio (`pe`), sem executar nada;
   fallback = ícone genérico. (GUI 1+; M0 não extrai ícone.)
