# Perfis de compatibilidade (por aplicação)

Arquivo `capsule.toml` por app (formato congelado v0.2):

```toml
[app]
name = "filetest"
windows_version = [10, 0]   # opcional; default [10, 0]
arch = "x86_64"             # informativo
current_dir = "C:\\work"    # opcional; default = CWD do host

[drives]
C = "/tmp/capsule-c"        # letra (case-insensitive) -> dir do host
D = "/mnt/dados"
```

Uso: `rine --capsule capsule.toml file.exe arg1`. Sem `--capsule`, drives
vazios (só paths relativos ao CWD) e cmdline = argv do host.
Erros de formato abortam o load (nunca default silencioso).
