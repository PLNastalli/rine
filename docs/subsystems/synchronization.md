# Sincronização

Princípio: mapear só quando a semântica permite (`futex`/`eventfd`/`epoll`
quando correto, nunca cego — ver exemplos no pedido original). v0.1: `Event`
em memória (`Mutex<bool>`, suficiente single-thread). v0.3: `NtCreateEvent`,
`NtWaitForSingleObject` com futex real, `Sleep`, seção crítica. Testes:
`threads.exe`/`sync.exe` (v0.3).
