/* oracle_probe.c — produtor Windows do formato OracleResult (schema 1).
 *
 * COMPILA aqui (MinGW, ver build_mingw.sh), RODA SÓ no Windows real.
 * Saída: um OracleResult JSON por linha no stdout. Nunca inventar valores:
 * tudo é medido (GetLastError, retornos, exit codes).
 *
 * Casos espelham `evil.exe`/`file.exe` do Rine para diff futuro:
 *   createfile-invalid-drive   CreateFileA("Z:\nope.txt", READ, OPEN_EXISTING)
 *   createfile-bad-disposition CreateFileA(<tmp>\probe.txt, READ, disp=9)
 *   createfile-roundtrip       create/write/close/reopen/read/verify
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

static void jstr(const char *s) {
    putchar('"');
    for (; *s; s++) {
        if (*s == '"' || *s == '\\') putchar('\\');
        if (*s == '\n') { printf("\\n"); continue; }
        if (*s == '\r') { printf("\\r"); continue; }
        putchar(*s);
    }
    putchar('"');
}

static void emit(const char *test, int exit_code, const char *rv, DWORD gle,
                 const char *extra /* JSON já pronto ou NULL */) {
    printf("{\"schema\":1,\"test\":");
    jstr(test);
    printf(",\"side\":\"windows\",\"producer_version\":null,"
           "\"architecture\":\"x86_64\",\"exit_code\":%d,\"crash\":null,"
           "\"stdout_bytes\":0,\"stderr_bytes\":0,"
           "\"stdout_text\":null,\"stderr_text\":null,"
           "\"observations\":{\"return_value\":");
    if (rv) jstr(rv); else printf("null");
    printf(",\"get_last_error\":%lu,\"ntstatus\":null},\"files_created\":[]",
           (unsigned long)gle);
    if (extra) printf(",%s", extra);
    printf("}\n");
}

int main(void) {
    char rv[32];
    HANDLE h;
    DWORD w;

    /* 1. drive inexistente */
    h = CreateFileA("Z:\\nope.txt", GENERIC_READ, 0, NULL, OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL, NULL);
    sprintf(rv, "%p", (void *)h);
    emit("createfile-invalid-drive", 0, rv, GetLastError(), NULL);
    if (h != INVALID_HANDLE_VALUE) CloseHandle(h);

    /* 2. disposition inválida */
    h = CreateFileA("C:\\Windows\\Temp\\rine_probe_disp.txt", GENERIC_READ, 0,
                    NULL, 9, FILE_ATTRIBUTE_NORMAL, NULL);
    sprintf(rv, "%p", (void *)h);
    emit("createfile-bad-disposition", 0, rv, GetLastError(), NULL);
    if (h != INVALID_HANDLE_VALUE) CloseHandle(h);

    /* 3. roundtrip */
    {
        char tmp[MAX_PATH], fp[MAX_PATH];
        const char *msg = "Hello File\n";
        char buf[64];
        DWORD r = 0;
        int ok = 0;
        GetTempPathA(sizeof tmp, tmp);
        snprintf(fp, sizeof fp, "%srine_probe_rt.txt", tmp);
        h = CreateFileA(fp, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS,
                        FILE_ATTRIBUTE_NORMAL, NULL);
        if (h != INVALID_HANDLE_VALUE) {
            if (WriteFile(h, msg, 11, &w, NULL) && w == 11) {
                CloseHandle(h);
                h = CreateFileA(fp, GENERIC_READ, 0, NULL, OPEN_EXISTING,
                                FILE_ATTRIBUTE_NORMAL, NULL);
                if (h != INVALID_HANDLE_VALUE
                    && ReadFile(h, buf, sizeof buf, &r, NULL) && r == 11
                    && memcmp(buf, msg, 11) == 0) {
                    ok = 1;
                }
            }
            if (h != INVALID_HANDLE_VALUE) CloseHandle(h);
        }
        DeleteFileA(fp);
        sprintf(rv, "%d", ok);
        emit("createfile-roundtrip", ok ? 0 : 1, rv, GetLastError(), NULL);
    }
    return 0;
}
