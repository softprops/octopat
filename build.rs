use std::{env, error::Error, fs, path::Path};

//use std::ascii::AsciiExt;

fn to_ident(scope: impl AsRef<str>) -> String {
    let mut buf = String::new();
    let mut capitalize = true;
    for ch in scope.as_ref().chars() {
        if ch == '_' || ch == '-' || ch == ':' {
            capitalize = true;
        } else if capitalize {
            buf.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            buf.push(ch);
        }
    }
    buf
}
fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("scope.rs");
    let mut content = "#[derive(Clone, Deserialize, Serialize, Copy, IntoEnumIterator, PartialEq)]
    pub enum Scope {
        "
    .to_string();
    for line in include_str!("data/scopes.txt").lines() {
        content.push_str(
            format!(
                "
        #[serde(rename = \"{name}\")]
        {variant},
        ",
                name = line,
                variant = to_ident(line)
            )
            .as_str(),
        );
    }
    content.push_str("}");
    fs::write(&dest_path, content)?;
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data/scopes.txt");
    Ok(())
}
