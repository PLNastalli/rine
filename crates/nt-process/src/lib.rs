//! `nt-process`: modelo de processo NT (PEB, parâmetros, ciclo de vida).
//!
//! v0.1: estrutura mínima observável + exit-code. Processo real (fork/
//! namespaces) no milestone v0.4; hoje o emulado vive dentro do host.

use std::sync::Mutex;
use winabi::PebMinimal;

#[derive(Debug, Clone)]
pub struct ProcessParameters {
    pub image_path: String,
    pub command_line: String,
    pub current_dir: String,
}

#[derive(Debug)]
pub struct Process {
    pub pid_win: u32,
    pub image_base: u64,
    pub peb: Mutex<PebMinimal>,
    pub params: ProcessParameters,
    pub exit_code: Mutex<Option<u32>>,
}

impl Process {
    pub fn new(pid_win: u32, image_base: u64, params: ProcessParameters) -> Self {
        Self {
            pid_win,
            image_base,
            peb: Mutex::new(PebMinimal {
                image_base,
                process_parameters: 0,
                loader_data: 0,
            }),
            params,
            exit_code: Mutex::new(None),
        }
    }

    pub fn terminate(&self, code: u32) {
        *self.exit_code.lock().unwrap() = Some(code);
    }

    pub fn exit_code(&self) -> Option<u32> {
        *self.exit_code.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminate_records_exit_code() {
        let p = Process::new(
            0x1000,
            0x0014_0000_0000,
            ProcessParameters {
                image_path: "hello.exe".into(),
                command_line: "hello.exe".into(),
                current_dir: "C:\\".into(),
            },
        );
        assert_eq!(p.exit_code(), None);
        p.terminate(3);
        assert_eq!(p.exit_code(), Some(3));
    }
}
