use lazy_static::lazy_static;

lazy_static! {
    static ref ROOT: String = {
        let path = std::env::current_dir();
        match path {
            Ok(p) => p.to_str().unwrap().to_owned(),
            Err(e) => {
                eprintln!("Failed to get current directory: {}", e);
                std::process::exit(1);
            }
        }
    };
}

pub const COSIGN_PATH: &str = "cosign.pub";
pub const MODULES_PATH: &str = "config/modules";
pub const RECIPE_PATH: &str = "config/recipe.yaml";
