//! Dogfood do parser ApiSet (dev-only, nunca em teste automatizado).
//!
//! Lê um `apisetschema.dll` Windows REAL (oracle local, fora do git) e
//! imprime o snapshot namespace→host como JSON (ver
//! `api-db/windows-11-25h2-x64/apiset-map.json`, gerado a partir daqui).
//!
//! ```bash
//! cargo run -q -p pe --example dump_apiset -- \
//!   windows-reference/win11-25h2/System32/apisetschema.dll
//! ```

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("uso: dump_apiset <apisetschema.dll>");
    let bytes = std::fs::read(&path).expect("ler apisetschema.dll");
    let img = pe::Image::parse(&bytes).expect("parse PE");
    let map = img
        .apiset_map()
        .expect("parse apiset")
        .expect("apisetschema.dll sem seção .apiset?!");
    println!("{{\"provenance\": \"pe::apiset sobre {path} (oracle, fora do git)\",");
    println!("\"count\": {},", map.len());
    println!("\"routes\": [");
    for (i, e) in map.iter().enumerate() {
        let comma = if i + 1 == map.len() { "" } else { "," };
        // Nomes vêm do parser (minúsculos, validados); aspas escapadas aqui.
        let ns = e.namespace.replace('\\', "\\\\").replace('"', "\\\"");
        let host = e.host.replace('\\', "\\\\").replace('"', "\\\"");
        println!("  {{\"namespace\": \"{ns}\", \"host\": \"{host}\"}}{comma}");
    }
    println!("]}}");
}
