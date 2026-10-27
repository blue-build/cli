use std::{
    env,
    fs::{self, DirEntry},
    path::PathBuf,
};

fn main() {
    println!("cargo:rerun-if-changed=scripts/");
    let entries = fs::read_dir("scripts/")
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let scripts = entries
        .iter()
        .filter(|entry| {
            entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "sh")
        })
        .map(DirEntry::path)
        .map(|path| {
            format!(
                r#"("{path}", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/{path}"))),"#,
                path = path.file_name().unwrap().display()
            )
        })
        .collect::<Vec<_>>();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("build_scripts.rs");
    fs::write(
        out_path,
        format!(
            "const BUILD_SCRIPTS: [(&str, &[u8]); {}] = [\n{}\n];",
            scripts.len(),
            scripts.join("\n"),
        ),
    )
    .unwrap();
}
