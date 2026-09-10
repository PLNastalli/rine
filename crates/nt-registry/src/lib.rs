//! `nt-registry`: registry virtual por Capsule.
//!
//! Árvore em memória (`HKLM/HKCU`) + persistência por-capsule (`registry.toml`).
//! A árvore em memória é o overlay; `save`/`load` serializam tudo.
//! APIs guest (`advapi32`) chegam em v0.3; o formato de arquivo congela em v0.2.

use std::collections::HashMap;
use std::sync::Mutex;
use winabi::NtStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Sz(String),
    Dword(u32),
    Qword(u64),
    Binary(Vec<u8>),
}

#[derive(Debug, Default)]
struct Key {
    values: HashMap<String, RegValue>,
    subkeys: HashMap<String, Key>,
}

#[derive(Debug, Default)]
pub struct Registry {
    root: Mutex<HashMap<String, Key>>,
}

impl Registry {
    pub fn new() -> Self {
        let mut root = HashMap::new();
        root.insert("HKLM".into(), Key::default());
        root.insert("HKCU".into(), Key::default());
        Self {
            root: Mutex::new(root),
        }
    }

    fn parent_mut<'a>(
        map: &'a mut HashMap<String, Key>,
        path: &[&str],
        create: bool,
    ) -> Result<&'a mut HashMap<String, Key>, NtStatus> {
        let mut cur: &'a mut HashMap<String, Key> = map;
        // Navega até o mapa-pai (todos os componentes menos o último).
        let (parents, _leaf) = path.split_at(path.len().saturating_sub(1));
        for comp in parents {
            if create {
                cur = &mut cur.entry(comp.to_string()).or_default().subkeys;
            } else {
                // Reborrow explícito para encadear sem alias.
                let next = cur.get_mut(*comp).ok_or(NtStatus::OBJECT_NAME_NOT_FOUND)?;
                cur = &mut next.subkeys;
            }
        }
        Ok(cur)
    }

    pub fn set(&self, path: &str, value_name: &str, value: RegValue) -> Result<(), NtStatus> {
        let comps: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        if comps.is_empty() {
            return Err(NtStatus::OBJECT_PATH_INVALID);
        }
        let mut root = self.root.lock().unwrap();
        let leaf = comps.last().unwrap().to_string();
        let parent = Self::parent_mut(&mut root, &comps, true)?;
        parent
            .entry(leaf)
            .or_default()
            .values
            .insert(value_name.to_string(), value);
        Ok(())
    }

    pub fn get(&self, path: &str, value_name: &str) -> Result<RegValue, NtStatus> {
        let comps: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        if comps.is_empty() {
            return Err(NtStatus::OBJECT_PATH_INVALID);
        }
        let mut root = self.root.lock().unwrap();
        let leaf = comps.last().unwrap().to_string();
        let parent = Self::parent_mut(&mut root, &comps, false)?;
        parent
            .get(&leaf)
            .and_then(|k| k.values.get(value_name))
            .cloned()
            .ok_or(NtStatus::OBJECT_NAME_NOT_FOUND)
    }

    /// Serializa tudo para `registry.toml` da Capsule. Formato (congelado v0.2):
    /// ```toml
    /// ["HKLM\\Software\\Rine"]
    /// Version = { s = "0.1" }
    /// Count = { dword = 3 }
    /// ```
    /// Chaves = paths com `\`; valores = tabelas de um campo (`s`/`dword`/
    /// `qword`/`bin` em hex minúsculo).
    pub fn save(&self, path: &str) -> Result<(), NtStatus> {
        let root = self.root.lock().unwrap();
        let mut out = toml::map::Map::new();
        let mut stack: Vec<(String, &Key)> = root.iter().map(|(k, v)| (k.clone(), v)).collect();
        while let Some((prefix, key)) = stack.pop() {
            if !key.values.is_empty() {
                let mut t = toml::map::Map::new();
                let mut names: Vec<&String> = key.values.keys().collect();
                names.sort();
                for n in names {
                    t.insert(n.clone(), reg_to_toml(&key.values[n]));
                }
                out.insert(prefix.clone(), toml::Value::Table(t));
            }
            for (sub, k) in &key.subkeys {
                stack.push((format!("{prefix}\\{sub}"), k));
            }
        }
        let text = toml::to_string(&toml::Value::Table(out)).map_err(|_| NtStatus::UNSUCCESSFUL)?;
        std::fs::write(path, text).map_err(|_| NtStatus::ACCESS_DENIED)
    }

    /// Carrega `registry.toml` (mescla sobre o conteúdo atual).
    pub fn load(&self, path: &str) -> Result<(), NtStatus> {
        let text = std::fs::read_to_string(path).map_err(|_| NtStatus::OBJECT_NAME_NOT_FOUND)?;
        let v: toml::Value = text.parse().map_err(|_| NtStatus::UNSUCCESSFUL)?;
        let table = v.as_table().ok_or(NtStatus::UNSUCCESSFUL)?;
        for (key_path, values) in table {
            let values = values.as_table().ok_or(NtStatus::UNSUCCESSFUL)?;
            for (name, tv) in values {
                self.set(key_path, name, toml_to_reg(tv)?)?;
            }
        }
        Ok(())
    }
}

fn reg_to_toml(v: &RegValue) -> toml::Value {
    let mut t = toml::map::Map::new();
    match v {
        RegValue::Sz(s) => {
            t.insert("s".into(), toml::Value::String(s.clone()));
        }
        RegValue::Dword(d) => {
            t.insert("dword".into(), toml::Value::Integer(*d as i64));
        }
        RegValue::Qword(q) => {
            t.insert("qword".into(), toml::Value::Integer(*q as i64));
        }
        RegValue::Binary(b) => {
            t.insert("bin".into(), toml::Value::String(hex_of(b)));
        }
    }
    toml::Value::Table(t)
}

fn toml_to_reg(v: &toml::Value) -> Result<RegValue, NtStatus> {
    let t = v.as_table().ok_or(NtStatus::UNSUCCESSFUL)?;
    if let Some(s) = t.get("s").and_then(|x| x.as_str()) {
        return Ok(RegValue::Sz(s.to_string()));
    }
    if let Some(d) = t.get("dword").and_then(|x| x.as_integer()) {
        return Ok(RegValue::Dword(d as u32));
    }
    if let Some(q) = t.get("qword").and_then(|x| x.as_integer()) {
        return Ok(RegValue::Qword(q as u64));
    }
    if let Some(b) = t.get("bin").and_then(|x| x.as_str()) {
        return Ok(RegValue::Binary(unhex(b)?));
    }
    Err(NtStatus::UNSUCCESSFUL)
}

fn hex_of(b: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push(H[(x >> 4) as usize] as char);
        s.push(H[(x & 15) as usize] as char);
    }
    s
}

fn unhex(s: &str) -> Result<Vec<u8>, NtStatus> {
    if s.len() % 2 != 0 {
        return Err(NtStatus::UNSUCCESSFUL);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| NtStatus::UNSUCCESSFUL))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rine-reg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("registry.toml");
        let path = f.to_string_lossy().into_owned();
        let r = Registry::new();
        r.set(
            "HKLM\\Software\\Rine",
            "Version",
            RegValue::Sz("0.2".into()),
        )
        .unwrap();
        r.set("HKLM\\Software\\Rine", "Count", RegValue::Dword(3))
            .unwrap();
        r.set("HKCU\\X", "Blob", RegValue::Binary(vec![0xDE, 0xAD]))
            .unwrap();
        r.save(&path).unwrap();

        let r2 = Registry::new();
        r2.load(&path).unwrap();
        assert_eq!(
            r2.get("HKLM\\Software\\Rine", "Version").unwrap(),
            RegValue::Sz("0.2".into())
        );
        assert_eq!(
            r2.get("HKLM\\Software\\Rine", "Count").unwrap(),
            RegValue::Dword(3)
        );
        assert_eq!(
            r2.get("HKCU\\X", "Blob").unwrap(),
            RegValue::Binary(vec![0xDE, 0xAD])
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
