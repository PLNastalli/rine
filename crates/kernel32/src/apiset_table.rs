//! Tabela ApiSet→módulo GERADA — não edite à mão.
//!
//! Fonte: `api-db/windows-11-25h2-x64/apiset-map.json` (snapshot do
//! ApiSetMap real via `pe::apiset`; hosts `kernel32/kernelbase/ntdll`).
//! Regra v0: `kernelbase.dll` roteia para `KnownModule::Kernel32` (nosso
//! kernel32 implementa a superfície Win32 unida; o split kernelbase é
//! detalhe interno — ver `docs/compatibility.md`). O teste
//! `apiset_table_matches_snapshot` trava esta tabela contra o fixture.
//!
//! Busca binária (tabela ordenada); custo irrelevante em load-time.

use super::KnownModule;

/// `(namespace minúsculo sem `.dll`, módulo)`, ORDENADO (binary_search).
pub(crate) const APISET_TABLE: &[(&str, KnownModule)] = &[
    ("api-ms-win-core-apiquery-l1-1-2", KnownModule::Ntdll),
    ("api-ms-win-core-apiquery-l2-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-appcompat-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-appinit-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-atoms-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-backgroundtask-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-calendar-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-comm-l1-1-2", KnownModule::Kernel32),
    (
        "api-ms-win-core-commandlinetoargv-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-console-ansi-l2-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-console-internal-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-console-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-console-l1-2-2", KnownModule::Kernel32),
    ("api-ms-win-core-console-l2-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-console-l2-2-0", KnownModule::Kernel32),
    ("api-ms-win-core-console-l3-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-console-l3-2-0", KnownModule::Kernel32),
    ("api-ms-win-core-crt-l1-1-0", KnownModule::Ntdll),
    ("api-ms-win-core-crt-l2-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-datetime-l1-1-2", KnownModule::Kernel32),
    ("api-ms-win-core-debug-l1-1-2", KnownModule::Kernel32),
    ("api-ms-win-core-delayload-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-enclave-l1-1-1", KnownModule::Kernel32),
    (
        "api-ms-win-core-errorhandling-l1-1-3",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-featuretoggles-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-fibers-l1-1-2", KnownModule::Kernel32),
    ("api-ms-win-core-fibers-l2-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-file-ansi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-file-ansi-l2-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-file-fromapp-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-file-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-file-l1-2-5", KnownModule::Kernel32),
    ("api-ms-win-core-file-l2-1-4", KnownModule::Kernel32),
    ("api-ms-win-core-firmware-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-guard-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-handle-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-heap-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-heap-l1-2-0", KnownModule::Kernel32),
    ("api-ms-win-core-heap-l2-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-heap-obsolete-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-interlocked-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-interlocked-l1-2-0", KnownModule::Kernel32),
    ("api-ms-win-core-io-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-ioring-l1-1-2", KnownModule::Kernel32),
    ("api-ms-win-core-job-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-job-l2-1-1", KnownModule::Kernel32),
    (
        "api-ms-win-core-kernel32-legacy-ansi-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-kernel32-legacy-l1-1-6",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-kernel32-private-l1-1-2",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-kernel32-private-l1-2-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-largeinteger-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-libraryloader-l1-1-1",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-libraryloader-l1-2-3",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-libraryloader-l2-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-libraryloader-private-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-localization-ansi-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-localization-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-localization-l1-2-4", KnownModule::Kernel32),
    ("api-ms-win-core-localization-l2-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-localization-obsolete-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-localization-obsolete-l1-2-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-localization-obsolete-l1-3-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-localization-private-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-localregistry-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-memory-l1-1-9", KnownModule::Kernel32),
    ("api-ms-win-core-misc-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-namedpipe-ansi-l1-1-1",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-namedpipe-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-namedpipe-l1-2-2", KnownModule::Kernel32),
    (
        "api-ms-win-core-namespace-ansi-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-namespace-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-normalization-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-path-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-pcw-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-perfcounters-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-perfcounters-l1-2-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-privateprofile-l1-1-1",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processenvironment-ansi-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processenvironment-l1-1-1",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processenvironment-l1-2-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processsecurity-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processsnapshot-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processthreads-l1-1-8",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processtopology-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processtopology-l1-2-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processtopology-obsolete-l1-1-1",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-processtopology-private-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-profile-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-psapi-ansi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-psapi-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-psapi-obsolete-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-psapiansi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-psm-key-l1-1-3", KnownModule::Kernel32),
    ("api-ms-win-core-quirks-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-realtime-l1-1-2", KnownModule::Kernel32),
    ("api-ms-win-core-registry-l1-1-2", KnownModule::Kernel32),
    (
        "api-ms-win-core-registryuserspecific-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-rtlsupport-l1-1-1", KnownModule::Ntdll),
    ("api-ms-win-core-rtlsupport-l1-2-2", KnownModule::Ntdll),
    (
        "api-ms-win-core-shlwapi-legacy-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-shlwapi-obsolete-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-shlwapi-obsolete-l1-2-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-sidebyside-ansi-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-sidebyside-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-state-helpers-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-string-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-string-l2-1-1", KnownModule::Kernel32),
    (
        "api-ms-win-core-string-obsolete-l1-1-1",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-stringansi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-stringloader-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-synch-ansi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-synch-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-synch-l1-2-1", KnownModule::Kernel32),
    ("api-ms-win-core-sysinfo-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-sysinfo-l1-2-8", KnownModule::Kernel32),
    (
        "api-ms-win-core-systemtopology-l1-1-2",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-threadpool-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-threadpool-l1-2-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-threadpool-legacy-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-core-threadpool-private-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-timezone-l1-1-1", KnownModule::Kernel32),
    (
        "api-ms-win-core-timezone-private-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-toolhelp-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-ums-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-url-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-core-util-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-version-l1-1-1", KnownModule::Kernel32),
    (
        "api-ms-win-core-version-private-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-versionansi-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-core-windowsceip-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-core-windowserrorreporting-l1-1-3",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-core-wow64-l1-1-3", KnownModule::Kernel32),
    ("api-ms-win-core-xstate-l1-1-3", KnownModule::Ntdll),
    ("api-ms-win-core-xstate-l2-1-2", KnownModule::Kernel32),
    (
        "api-ms-win-deprecated-apis-obsolete-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-downlevel-advapi32-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-downlevel-kernel32-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-downlevel-kernel32-l2-1-0",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-downlevel-normaliz-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-downlevel-shlwapi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-downlevel-user32-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-downlevel-version-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-eventing-classicprovider-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-eventing-provider-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-gaming-deviceinformation-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-http-time-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-legacy-shlwapi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-mm-time-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-obsolete-localization-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-obsolete-psapi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-obsolete-shlwapi-l1-1-0", KnownModule::Kernel32),
    ("api-ms-win-oobe-notification-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-security-activedirectoryclient-l1-1-1",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-security-appcontainer-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-security-base-l1-1-1", KnownModule::Kernel32),
    ("api-ms-win-security-base-l1-2-2", KnownModule::Kernel32),
    (
        "api-ms-win-security-base-private-l1-1-2",
        KnownModule::Kernel32,
    ),
    (
        "api-ms-win-security-grouppolicy-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("api-ms-win-shell-shellcom-l1-1-0", KnownModule::Kernel32),
    (
        "api-ms-win-stateseparation-helpers-l1-1-1",
        KnownModule::Kernel32,
    ),
    (
        "ext-ms-win-kernel32-appcompat-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("ext-ms-win-kernel32-datetime-l1-1-0", KnownModule::Kernel32),
    (
        "ext-ms-win-kernel32-elevation-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "ext-ms-win-kernel32-errorhandling-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("ext-ms-win-kernel32-file-l1-1-0", KnownModule::Kernel32),
    (
        "ext-ms-win-kernel32-localization-l1-1-0",
        KnownModule::Kernel32,
    ),
    ("ext-ms-win-kernel32-process-l1-1-0", KnownModule::Kernel32),
    ("ext-ms-win-kernel32-quirks-l1-1-1", KnownModule::Kernel32),
    ("ext-ms-win-kernel32-registry-l1-1-0", KnownModule::Kernel32),
    (
        "ext-ms-win-kernel32-sidebyside-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "ext-ms-win-kernel32-transacted-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "ext-ms-win-kernel32-updateresource-l1-1-0",
        KnownModule::Kernel32,
    ),
    (
        "ext-ms-win-kernel32-windowserrorreporting-l1-1-1",
        KnownModule::Kernel32,
    ),
    (
        "ext-ms-win-kernelbase-processthread-l1-1-3",
        KnownModule::Kernel32,
    ),
    (
        "ext-ms-win-kernelbase-processthread-l1-2-0",
        KnownModule::Kernel32,
    ),
];

/// Namespace ApiSet → módulo implementado (`None` = outro host — UCRT,
/// USER32, … — ou nome fora do namespace; o caller vira `MOD_NOT_FOUND`).
pub fn apiset_host(dll: &str) -> Option<KnownModule> {
    let lower = dll.to_ascii_lowercase();
    let key = lower.strip_suffix(".dll").unwrap_or(&lower);
    APISET_TABLE
        .binary_search_by(|(n, _)| n.cmp(&key))
        .ok()
        .map(|i| APISET_TABLE[i].1)
}
