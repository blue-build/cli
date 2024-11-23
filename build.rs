use shadow_rs::SdResult;

fn main() -> SdResult<()> {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    shadow_rs::new()
}
