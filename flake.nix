{
  
  description = "BlueBuild's command line program that builds Containerfiles and custom images";

  
  inputs = {
    flake-schemas.url = "https://flakehub.com/f/DeterminateSystems/flake-schemas/*.tar.gz";

    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.0.tar.gz";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  
  outputs = { self, flake-schemas, nixpkgs, rust-overlay }:
    let
      overlays = [
        rust-overlay.overlays.default
        (final: prev: {
          rustToolchain = (final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override { extensions = [ "rust-src"]; };
        })
      ];
      
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forEachSupportedSystem = f: nixpkgs.lib.genAttrs supportedSystems (system: f rec {
        pkgs = import nixpkgs { inherit overlays system; };
        lib = pkgs.lib;
      });
    in {      
      schemas = flake-schemas.schemas;

      packages = forEachSupportedSystem ({ pkgs, lib }: rec {
        default = bluebuild;
        bluebuild = pkgs.rustPlatform.buildRustPackage rec {
          pname = "bluebuild";
          version = "v0.8.1";

          src = pkgs.fetchFromGitHub {
            owner = "blue-build";
            repo = "cli";
            rev = version;
            sha256 = "07mw9d8xn6gxcar793mw9jwchq4fxh7c3739ybb9myasqq0279mk";
          };

          cargoSha256 = "sha256-rVU9ZdBr9Z3qGavik4kwtifxJL0U0xaC7J+a+YbiSgA=";

          meta = {
            description = "BlueBuild's command line program that builds Containerfiles and custom images";
            homepage = "https://github.com/blue-build/cli";
            license = lib.licenses.apsl20;
          };
        };
      });
      
      devShells = forEachSupportedSystem ({ pkgs, ... }: {
        default = pkgs.mkShell {
          
          packages = with pkgs; [
            rustToolchain
            cargo-bloat
            cargo-edit
            cargo-outdated
            cargo-watch
            rust-analyzer
            cargo
            rustc
            bacon
            earthly
            yq
            jq
            nixpkgs-fmt
          ];

          env = {
            RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
          };
        };
      });
    };
}
