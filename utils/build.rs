use std::env;
use std::path::PathBuf;
use syntect::dumps;
use syntect::parsing::syntax_definition::SyntaxDefinition;
use syntect::parsing::SyntaxSetBuilder;

fn main() {
    let mut ssb = SyntaxSetBuilder::new();
    ssb.add(
        SyntaxDefinition::load_from_str(
            include_str!("highlights/Dockerfile.sublime-syntax"),
            true,
            None,
        )
        .unwrap(),
    );
    let ss = ssb.build();

    dumps::dump_to_uncompressed_file(
        &ss,
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("docker_syntax.bin"),
    )
    .unwrap();
    println!("cargo:rerun-if-changed=highlights/Dockerfile.sublime-syntax");
}
