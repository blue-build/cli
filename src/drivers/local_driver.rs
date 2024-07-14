use super::CiDriver;

pub struct LocalDriver;

impl CiDriver for LocalDriver {
    fn on_default_branch() -> bool {
        todo!()
    }

    fn keyless_cert_identity() -> miette::Result<String> {
        todo!()
    }

    fn generate_tags(_recipe: &blue_build_recipe::Recipe) -> miette::Result<Vec<String>> {
        todo!()
    }

    fn get_repo_url() -> miette::Result<String> {
        todo!()
    }

    fn get_registry() -> miette::Result<String> {
        todo!()
    }
}
