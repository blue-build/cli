use constcat::concat;

// Paths
pub const ARCHIVE_SUFFIX: &str = "tar.gz";
pub const CONFIG_PATH: &str = "./config";
pub const CONTAINERFILES_PATH: &str = "./containerfiles";
pub const CONTAINER_FILE: &str = "Containerfile";
pub const COSIGN_PUB_PATH: &str = "./cosign.pub";
pub const COSIGN_PRIV_PATH: &str = "./cosign.key";
pub const FILES_PATH: &str = "./files";
pub const LOCAL_BUILD: &str = "/etc/bluebuild";
pub const MODULES_PATH: &str = "./config/modules";
pub const RECIPE_FILE: &str = "recipe.yml";
pub const RECIPE_PATH: &str = "./recipes";

// Labels
pub const BUILD_ID_LABEL: &str = "org.blue-build.build-id";
pub const IMAGE_VERSION_LABEL: &str = "org.opencontainers.image.version";

// BlueBuild vars
pub const BB_CACHE_LAYERS: &str = "BB_CACHE_LAYERS";
pub const BB_PASSWORD: &str = "BB_PASSWORD";
pub const BB_PRIVATE_KEY: &str = "BB_PRIVATE_KEY";
pub const BB_REGISTRY: &str = "BB_REGISTRY";
pub const BB_REGISTRY_NAMESPACE: &str = "BB_REGISTRY_NAMESPACE";
pub const BB_USERNAME: &str = "BB_USERNAME";
pub const BB_BUILD_RECHUNK: &str = "BB_BUILD_RECHUNK";
pub const BB_BUILD_RECHUNK_CLEAR_PLAN: &str = "BB_BUILD_RECHUNK_CLEAR_PLAN";

// Docker vars
pub const DOCKER_HOST: &str = "DOCKER_HOST";

// Cosign vars
pub const COSIGN_PASSWORD: &str = "COSIGN_PASSWORD";
pub const COSIGN_PRIVATE_KEY: &str = "COSIGN_PRIVATE_KEY";
pub const COSIGN_YES: &str = "COSIGN_YES";
pub const GITHUB_TOKEN_ISSUER_URL: &str = "https://token.actions.githubusercontent.com";
pub const SIGSTORE_ID_TOKEN: &str = "SIGSTORE_ID_TOKEN";

// GitHub CI vars
pub const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";
pub const GITHUB_ACTOR: &str = "GITHUB_ACTOR";
pub const GITHUB_EVENT_NAME: &str = "GITHUB_EVENT_NAME";
pub const GITHUB_EVENT_PATH: &str = "GITHUB_EVENT_PATH";
pub const GITHUB_REF_NAME: &str = "GITHUB_REF_NAME";
pub const GITHUB_RESPOSITORY: &str = "GITHUB_REPOSITORY";
pub const GITHUB_REPOSITORY_OWNER: &str = "GITHUB_REPOSITORY_OWNER";
pub const GITHUB_SERVER_URL: &str = "GITHUB_SERVER_URL";
pub const GITHUB_SHA: &str = "GITHUB_SHA";
pub const GITHUB_TOKEN: &str = "GH_TOKEN";
pub const GITHUB_WORKFLOW_REF: &str = "GITHUB_WORKFLOW_REF";
pub const PR_EVENT_NUMBER: &str = "GH_PR_EVENT_NUMBER";

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
pub const GITLAB_CI: &str = "GITLAB_CI";

// Terminal vars
pub const TERM_PROGRAM: &str = "TERM_PROGRAM";
pub const LC_TERMINAL: &str = "LC_TERMINAL";
pub const TERM_PROGRAM_VERSION: &str = "TERM_PROGRAM_VERSION";
pub const LC_TERMINAL_VERSION: &str = "LC_TERMINAL_VERSION";
pub const XDG_RUNTIME_DIR: &str = "XDG_RUNTIME_DIR";
pub const SUDO_ASKPASS: &str = "SUDO_ASKPASS";

// Misc
pub const BLUE_BUILD: &str = "bluebuild";
pub const BUILD_SCRIPTS_IMAGE_REF: &str = "ghcr.io/blue-build/cli/build-scripts";
pub const BLUE_BULID_IMAGE_REF: &str = "ghcr.io/blue-build/cli";
pub const BLUE_BUILD_MODULE_IMAGE_REF: &str = "ghcr.io/blue-build/modules";
pub const COSIGN_IMAGE: &str = "ghcr.io/sigstore/cosign/cosign:v2.5.2";
pub const NUSHELL_IMAGE: &str = "ghcr.io/blue-build/nushell-image";
pub const OCI_ARCHIVE: &str = "oci-archive";
pub const OSTREE_IMAGE_SIGNED: &str = "ostree-image-signed";
pub const OSTREE_UNVERIFIED_IMAGE: &str = "ostree-unverified-image";
pub const SKOPEO_IMAGE: &str = "quay.io/skopeo/stable:latest";
pub const TEMPLATE_REPO_URL: &str = "https://github.com/blue-build/template.git";
pub const USER: &str = "USER";
pub const UNKNOWN_SHELL: &str = "<unknown shell>";
pub const UNKNOWN_VERSION: &str = "<unknown version>";
pub const UNKNOWN_TERMINAL: &str = "<unknown terminal>";
pub const GITHUB_CHAR_LIMIT: usize = 8100; // Magic number accepted by Github

// Schema
pub const SCHEMA_BASE_URL: &str = "https://schema.blue-build.org";
pub const RECIPE_V1_SCHEMA_URL: &str = concat!(SCHEMA_BASE_URL, "/recipe-v1.json");
pub const STAGE_V1_SCHEMA_URL: &str = concat!(SCHEMA_BASE_URL, "/stage-v1.json");
pub const MODULE_V1_SCHEMA_URL: &str = concat!(SCHEMA_BASE_URL, "/module-v1.json");
pub const MODULE_STAGE_LIST_V1_SCHEMA_URL: &str =
    concat!(SCHEMA_BASE_URL, "/module-stage-list-v1.json");

// JSON Schema
pub const JSON_SCHEMA: &str = "json-schema://";
pub const CUSTOM_MODULE_SCHEMA: &str = concat!(JSON_SCHEMA, "/module-custom-v1.json");
pub const IMPORT_MODULE_SCHEMA: &str = concat!(JSON_SCHEMA, "/import-v1.json");
pub const STAGE_SCHEMA: &str = concat!(JSON_SCHEMA, "/stage-v1.json");

// Messages
pub const BUG_REPORT_WARNING_MESSAGE: &str =
    "Please copy the above report and open an issue manually.";
