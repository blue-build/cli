use shadow_rs::SdResult;
use std::fs::File;
use std::io::Write;
use std::process::Command;

fn main() -> SdResult<()> {
    shadow_rs::new_hook(hook)
}

fn hook(file: &File) -> SdResult<()> {
    append_write_const(file)?;
    Ok(())
}

fn append_write_const(mut file: &File) -> SdResult<()> {
    let hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|x| String::from_utf8(x.stdout).ok())
        .map(|x| x.map(|x| x.trim().to_string()))
        .unwrap_or(None);

    let short_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|x| String::from_utf8(x.stdout).ok())
        .map(|x| x.map(|x| x.trim().to_string()))
        .unwrap_or(None);

    let hook_const: &str = &format!(
        "{}\n{}",
        &format!(
            "pub const BB_COMMIT_HASH: &str = \"{}\";",
            hash.unwrap_or_default()
        ),
        &format!(
            "pub const BB_COMMIT_HASH_SHORT: &str = \"{}\";",
            short_hash.unwrap_or_default()
        )
    );

    writeln!(file, "{hook_const}")?;
    Ok(())
}
