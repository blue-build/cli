use super::SigningDriver;

pub struct SigstoreDriver;

impl SigningDriver for SigstoreDriver {
    fn generate_key_pair() -> miette::Result<()> {
        todo!()
    }

    fn check_signing_files() -> miette::Result<()> {
        todo!()
    }

    fn sign(image_digest: &str, key_arg: Option<String>) -> miette::Result<()> {
        todo!()
    }

    fn verify(image_name_tag: &str, verify_type: super::VerifyType) -> miette::Result<()> {
        todo!()
    }

    fn signing_login() -> miette::Result<()> {
        todo!()
    }
}
