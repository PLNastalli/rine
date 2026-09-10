# Exceções (SEH ↔ signals)

Plano desde o dia 1 (nenhuma decisão v0.1 o impossibilita): signals Linux
viram `EXCEPTION_RECORD` (`SIGSEGV→ACCESS_VIOLATION`,
`SIGFPE→INT_DIVIDE_BY_ZERO`, `SIGILL→ILLEGAL_INSTRUCTION`), despachados pela
chain `TEB.ExceptionList` + `.pdata` unwind. v0.1: tipos + códigos. v0.3:
`signalfd`/handlers + `__try/__except` mínimo + `seh.exe`.
`VectoredExceptionHandling` depois.
