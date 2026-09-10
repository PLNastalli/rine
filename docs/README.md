# Rine — docs

Índice da memória persistente do projeto. Um novo desenvolvedor ou sessão de IA
**sem contexto anterior** deve conseguir continuar só lendo estes arquivos.

| Arquivo | Responde |
|---|---|
| `current-state.md` | Onde estamos? O que falta? Próximo passo? |
| `architecture.md` | Como o sistema é DE FATO hoje? |
| `roadmap.md` | Milestones, critérios, o que NÃO fazer ainda |
| `compatibility.md` | Cobertura de APIs e método de medição |
| `safety-model.md` | Onde fica `unsafe` e por quê é correto |
| `security-model.md` | Threat model: o que é/não é contido hoje |
| `crash-format.md` | Formato `RINE-CRASH` (nunca só "Segmentation fault") |
| `coding-standards.md` | Convenções de código Rust |
| `testing-strategy.md` | Differential testing e harnesses |
| `glossary.md` | Vocabulário único do projeto |
| `dependency-map.md` | Dependências entre crates (sem ciclos) |
| `subsystem-map.md` | Quem implementa o quê, onde ficam os testes |
| `cleanup-policy.md` | Política standing: sem camadas mortas (substituir→migrar→deletar) |
| `subsystems/*` | Um arquivo por subsistema (invariantes, diagramas) |
| `adr/*` | Decisões arquiteturais registradas |
| `milestones/*` | Escopo e critérios por milestone |
| `development-log/*` | O que mudou, por quê, próximo passo |

Regra de finalização de tarefa: código + testes + `fmt`/`clippy`/`test` +
docs atualizadas + `current-state.md` atualizado + ADRs se houve decisão.
