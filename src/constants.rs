// Paths
pub const ARCHIVE_SUFFIX: &str = "tar.gz";
pub const COSIGN_PATH: &str = "./cosign.pub";
pub const LOCAL_BUILD: &str = "/etc/bluebuild";
pub const MODULES_PATH: &str = "./config/modules";
pub const RECIPE_PATH: &str = "./config/recipe.yml";
pub const RUN_PODMAN_SOCK: &str = "/run/podman/podman.sock";
pub const VAR_RUN_PODMAN_PODMAN_SOCK: &str = "/var/run/podman/podman.sock";
pub const VAR_RUN_PODMAN_SOCK: &str = "/var/run/podman.sock";

// Labels
pub const BUILD_ID_LABEL: &str = "org.blue-build.build-id";

// Cosign vars
pub const COSIGN_PRIVATE_KEY: &str = "COSIGN_PRIVATE_KEY";
pub const GITHUB_TOKEN_ISSUER_URL: &str = "https://token.actions.githubusercontent.com";
pub const SIGSTORE_ID_TOKEN: &str = "SIGSTORE_ID_TOKEN";

// GitHub CI vars
pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";
pub const GITHUB_ACTOR: &str = "GITHUB_ACTOR";
pub const GITHUB_EVENT_NAME: &str = "GITHUB_EVENT_NAME";
pub const GITHUB_REF_NAME: &str = "GITHUB_REF_NAME";
pub const GITHUB_REPOSITORY_OWNER: &str = "GITHUB_REPOSITORY_OWNER";
pub const GITHUB_SHA: &str = "GITHUB_SHA";
pub const GITHUB_TOKEN: &str = "GITHUB_TOKEN";
pub const GITHUB_WORKFLOW_REF: &str = "GITHUB_WORKFLOW_REF";
pub const PR_EVENT_NUMBER: &str = "PR_EVENT_NUMBER";

// GitLab CI vars
pub const CI_COMMIT_REF_NAME: &str = "CI_COMMIT_REF_NAME";
pub const CI_COMMIT_SHORT_SHA: &str = "CI_COMMIT_SHORT_SHA";
pub const CI_DEFAULT_BRANCH: &str = "CI_DEFAULT_BRANCH";
pub const CI_MERGE_REQUEST_IID: &str = "CI_MERGE_REQUEST_IID";
pub const CI_PIPELINE_SOURCE: &str = "CI_PIPELINE_SOURCE";
pub const CI_PROJECT_NAME: &str = "CI_PROJECT_NAME";
pub const CI_PROJECT_NAMESPACE: &str = "CI_PROJECT_NAMESPACE";
pub const CI_PROJECT_URL: &str = "CI_PROJECT_URL";
pub const CI_SERVER_HOST: &str = "CI_SERVER_HOST";
pub const CI_SERVER_PROTOCOL: &str = "CI_SERVER_PROTOCOL";
pub const CI_REGISTRY: &str = "CI_REGISTRY";
pub const CI_REGISTRY_PASSWORD: &str = "CI_REGISTRY_PASSWORD";
pub const CI_REGISTRY_USER: &str = "CI_REGISTRY_USER";

// Terminal vars
pub const TERM_PROGRAM: &str = "TERM_PROGRAM";
pub const LC_TERMINAL: &str = "LC_TERMINAL";
pub const TERM_PROGRAM_VERSION: &str = "TERM_PROGRAM_VERSION";
pub const LC_TERMINAL_VERSION: &str = "LC_TERMINAL_VERSION";
pub const XDG_RUNTIME_DIR: &str = "XDG_RUNTIME_DIR";

// Misc
pub const UNKNOWN_SHELL: &str = "<unknown shell>";
pub const UNKNOWN_VERSION: &str = "<unknown version>";
pub const UNKNOWN_TERMINAL: &str = "<unknown terminal>";
pub const GITHUB_CHAR_LIMIT: usize = 8100; // Magic number accepted by Github
