# Quirks (workarounds app-específicos documentados)

Proibido `if exe == ...` no núcleo. Quirk exige: ID, descrição, apps afetadas,
razão, teste que demonstra, data, condição de remoção. Formato:

```toml
[quirk.QUI-0001]
description = "PAGE_GUARD tratado como committed normal"
apps = ["* (global até v0.3)"]
reason = "sem vetor de guarda via mprotect puro"
introduced = "2026-09-10"
remove_when = "guard pages reais (v0.3, SEH)"
test = "nt-memory unit (futuro: guard.exe diferencial)"
```

Registro atual: QUI-0001 (acima). Novos quirks só com teste reproduzindo.
