# Processo

`nt-process::Process { pid_win, image_base, peb, params, exit_code }`
(vocabulário coberto por teste; construção plena com isolamento em v0.4).
v0.2: um processo emulado dentro do host; `NtTerminateProcess` atual =
registra código + `exit` do host (ponto único em `ntdll`); cmdline UTF-16 +
params block (`UNICODE_STRING` em 0x70) observáveis via PEB (`args.exe` prova).
`Capsule` (`capsule.toml`, ADR-0008) + `DriveMap` por processo (ADR-0007).
