use super::CiDriver;

pub struct GithubDriver;

impl CiDriver for GithubDriver {
    fn on_main_branch() -> bool {
        todo!()
    }

    fn cert_identity() -> miette::Result<String> {
        todo!()
    }

    fn generate_tags<T, S>(
        _recipe: &blue_build_recipe::Recipe,
        _alt_tags: Option<T>,
    ) -> miette::Result<Vec<String>>
    where
        T: AsRef<[S]>,
        S: AsRef<str>,
    {
        todo!()
    }

    fn get_repo_url() -> miette::Result<String> {
        todo!()
    }

    fn get_registry() -> miette::Result<String> {
        todo!()
    }
}
