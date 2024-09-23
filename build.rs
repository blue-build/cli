use shadow_rs::SdResult;

fn main() -> SdResult<()> {
    println!("cargo:rerun-if-changed=.git/HEAD");
    shadow_rs::new()
}
