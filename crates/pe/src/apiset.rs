//! Parser do ApiSetMap (`API_SET_NAMESPACE`, versão 6).
//!
//! O schema ApiSet (`apisetschema.dll`, seção `.apiset`) roteia nomes
//! `api-ms-win-*`/`ext-ms-*` para a DLL hospedeira real (`kernel32.dll`,
//! `ucrtbase.dll`, …). É formato de dados (fatos de roteamento, não código)
//! documentado por pesquisa pública; extraímos só pares
//! namespace→host para o resolvedor (`kernel32::modules`).
//!
//! Layout (tudo relativo ao início do namespace, offsets em bytes):
//! header: Version u32 (=6), Size u32, Flags u32, Count u32, EntryOffset u32.
//! entrada (24B): Flags, NameOffset, NameLength, HashedLength, ValueOffset,
//! ValueCount (u32s). valor (24B): Flags, NameOffset, NameLength,
//! ValueOffset, ValueLength, _reserved. Nomes em UTF-16 (sem NUL).
//! v0: primeiro valor de cada entrada (seleção por host/seal = futuro).

use crate::{Image, PeError};

/// Uma rota do namespace (`api-ms-win-core-synch-l1-1-0` → `kernel32.dll`).
/// Ambos minúsculos; namespace sem sufixo `.dll` (como no mapa real).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiSetEntry {
    /// Namespace (`api-ms-win-crt-heap-l1-1-0`).
    pub namespace: String,
    /// Hospedeira (`ucrtbase.dll`).
    pub host: String,
}

fn u32at(data: &[u8], base: usize, off: u32) -> Result<u32, PeError> {
    let at = base
        .checked_add(off as usize)
        .ok_or(PeError::BadApiSet("offset overflow"))?;
    let end = at
        .checked_add(4)
        .ok_or(PeError::BadApiSet("offset overflow"))?;
    data.get(at..end)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(PeError::BadApiSet("namespace truncado"))
}

fn utf16at(data: &[u8], base: usize, off: u32, len: u32) -> Result<String, PeError> {
    let at = base
        .checked_add(off as usize)
        .ok_or(PeError::BadApiSet("string offset overflow"))?;
    let end = at
        .checked_add(len as usize)
        .ok_or(PeError::BadApiSet("string len overflow"))?;
    let bytes = data
        .get(at..end)
        .ok_or(PeError::BadApiSet("string truncada"))?;
    if bytes.len() % 2 != 0 {
        return Err(PeError::BadApiSet("utf-16 ímpar"));
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    // Estrito: surrogates/unpaired viram erro, nunca substituição silenciosa.
    String::from_utf16(&units).map_err(|_| PeError::BadApiSet("utf-16 inválido"))
}

/// Parseia o namespace a partir de `data[base..]` (bytes da seção `.apiset`).
/// Regras v0 (documentadas porque são decisão, não acidente):
/// - primeiro valor de cada entrada (hosts alternativos/seal = futuro);
/// - valor vazio → entrada pulada (`ext-ms-*`, seladas e deprecated não
///   roteiam em v0; os valores seguintes de entradas multi-valoradas têm
///   layout de alias — ler além do primeiro interpreta lixo).
pub fn parse_namespace(data: &[u8], base: usize) -> Result<Vec<ApiSetEntry>, PeError> {
    let ver = u32at(data, base, 0)?;
    if ver != 6 {
        return Err(PeError::BadApiSet("versão != 6"));
    }
    let count = u32at(data, base, 12)?;
    let entry_off = u32at(data, base, 16)? as usize;
    let mut out = Vec::new();
    for i in 0..count {
        let eo = entry_off
            .checked_add(
                (i as usize)
                    .checked_mul(24)
                    .ok_or(PeError::BadApiSet("entrada overflow"))?,
            )
            .ok_or(PeError::BadApiSet("entrada overflow"))?;
        let eb = base
            .checked_add(eo)
            .ok_or(PeError::BadApiSet("entrada overflow"))?;
        // Garante os 24B da entrada antes de ler qualquer campo.
        if data
            .get(
                eb..eb
                    .checked_add(24)
                    .ok_or(PeError::BadApiSet("entrada overflow"))?,
            )
            .is_none()
        {
            return Err(PeError::BadApiSet("entrada truncada"));
        }
        let name = utf16at(data, base, u32at(data, eb, 4)?, u32at(data, eb, 8)?)?;
        let vcount = u32at(data, eb, 20)?;
        if vcount == 0 {
            continue;
        }
        let vo = u32at(data, eb, 16)? as usize;
        let vb = base
            .checked_add(vo)
            .ok_or(PeError::BadApiSet("valor overflow"))?;
        if data
            .get(
                vb..vb
                    .checked_add(24)
                    .ok_or(PeError::BadApiSet("valor overflow"))?,
            )
            .is_none()
        {
            return Err(PeError::BadApiSet("valor truncado"));
        }
        // v0: primeiro valor (ver doc acima); vazio = entrada pulada.
        let host = utf16at(data, base, u32at(data, vb, 12)?, u32at(data, vb, 16)?)?;
        if host.is_empty() {
            continue;
        }
        let namespace = name
            .to_ascii_lowercase()
            .strip_suffix(".dll")
            .unwrap_or(&name.to_ascii_lowercase())
            .to_string();
        out.push(ApiSetEntry {
            namespace,
            host: host.to_ascii_lowercase(),
        });
    }
    Ok(out)
}

impl<'a> Image<'a> {
    /// Extrae o ApiSetMap da seção `.apiset` (`None` = PE sem a seção —
    /// normal fora de `apisetschema.dll`; erro = seção presente mas inválida).
    pub fn apiset_map(&self) -> Result<Option<Vec<ApiSetEntry>>, PeError> {
        let Some(sec) = self.sections.iter().find(|s| s.name == ".apiset") else {
            return Ok(None);
        };
        let start = sec.raw_offset as usize;
        let end = start
            .checked_add(sec.raw_size as usize)
            .ok_or(PeError::BadApiSet("seção overflow"))?;
        if self.data.get(start..end).is_none() {
            return Err(PeError::BadApiSet("seção fora do arquivo"));
        }
        parse_namespace(self.data, start).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16s(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
    }

    /// Namespace mínimo hand-built: 2 entradas × 1 valor (offsets calculados
    /// à mão; qualquer deriva do layout quebra aqui de propósito).
    fn minimal_blob() -> Vec<u8> {
        let n1 = u16s("api-ms-win-core-synch-l1-1-0");
        let h1 = u16s("kernel32.dll");
        let n2 = u16s("api-ms-win-crt-heap-l1-1-0");
        let h2 = u16s("ucrtbase.dll");
        // header(20) + 2 entradas(48) + 2 valores(48) + strings.
        let e_off = 20usize;
        let v_off = e_off + 48;
        let s_off = v_off + 48;
        let mut b = Vec::new();
        b.extend_from_slice(&6u32.to_le_bytes()); // Version
        b.extend_from_slice(&0u32.to_le_bytes()); // Size (não validado)
        b.extend_from_slice(&0u32.to_le_bytes()); // Flags
        b.extend_from_slice(&2u32.to_le_bytes()); // Count
        b.extend_from_slice(&(e_off as u32).to_le_bytes()); // EntryOffset
                                                            // entrada 1
        b.extend_from_slice(&0u32.to_le_bytes()); // Flags
        b.extend_from_slice(&(s_off as u32).to_le_bytes());
        b.extend_from_slice(&(n1.len() as u32).to_le_bytes());
        b.extend_from_slice(&(n1.len() as u32).to_le_bytes()); // HashedLength
        b.extend_from_slice(&(v_off as u32).to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes()); // ValueCount
                                                  // entrada 2
        let n2_off = s_off + n1.len() + h1.len();
        let v2_off = v_off + 24;
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&(n2_off as u32).to_le_bytes());
        b.extend_from_slice(&(n2.len() as u32).to_le_bytes());
        b.extend_from_slice(&(n2.len() as u32).to_le_bytes());
        b.extend_from_slice(&(v2_off as u32).to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        // valor 1 (host de n1)
        let h1_off = s_off + n1.len();
        b.extend_from_slice(&0u32.to_le_bytes()); // Flags
        b.extend_from_slice(&0u32.to_le_bytes()); // NameOffset (vazio)
        b.extend_from_slice(&0u32.to_le_bytes()); // NameLength
        b.extend_from_slice(&(h1_off as u32).to_le_bytes());
        b.extend_from_slice(&(h1.len() as u32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // reserved
                                                  // valor 2 (host de n2)
        let h2_off = n2_off + n2.len();
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&(h2_off as u32).to_le_bytes());
        b.extend_from_slice(&(h2.len() as u32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // strings: n1 h1 n2 h2
        b.extend_from_slice(&n1);
        b.extend_from_slice(&h1);
        b.extend_from_slice(&n2);
        b.extend_from_slice(&h2);
        b
    }

    #[test]
    fn minimal_namespace_roundtrip() {
        let entries = parse_namespace(&minimal_blob(), 0).unwrap();
        assert_eq!(
            entries,
            vec![
                ApiSetEntry {
                    namespace: "api-ms-win-core-synch-l1-1-0".into(),
                    host: "kernel32.dll".into(),
                },
                ApiSetEntry {
                    namespace: "api-ms-win-crt-heap-l1-1-0".into(),
                    host: "ucrtbase.dll".into(),
                },
            ]
        );
    }

    #[test]
    fn malformed_is_rejected_not_guessed() {
        assert!(parse_namespace(&[], 0).is_err());
        assert!(parse_namespace(&[6, 0, 0, 0], 0).is_err()); // header cortado
        let mut bad_ver = minimal_blob();
        bad_ver[0] = 5;
        assert!(parse_namespace(&bad_ver, 0).is_err());
        let mut trunc = minimal_blob();
        trunc.truncate(trunc.len() - 4);
        assert!(parse_namespace(&trunc, 0).is_err());
        // UTF-16 com surrogate solitário é rejeitado (nunca � silencioso).
        let mut bad_utf = minimal_blob();
        let pos = bad_utf.len() - 2;
        bad_utf[pos] = 0x00;
        bad_utf[pos + 1] = 0xD8;
        assert!(parse_namespace(&bad_utf, 0).is_err());
    }

    #[test]
    fn pe_without_apiset_section_is_none() {
        let hello = crate::builder::build_minimal_hello();
        let img = Image::parse(&hello).unwrap();
        assert!(img.apiset_map().unwrap().is_none());
    }
}
