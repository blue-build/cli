use shadow_rs::ShadowBuilder;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    ShadowBuilder::builder().build().unwrap();
}
