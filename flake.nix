{
  description = "A devShell example";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;
          };
        };

        # Cross-compilation setup for x86_64 musl
        pkgsMusl = import nixpkgs {
          inherit overlays;
          system = "x86_64-linux";
          crossSystem = {
            config = "x86_64-unknown-linux-musl";
          };
        };
      in
      {
        packages = {
          # Default build for current platform
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "kueue-dev";
            version = "0.5.6";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            buildInputs = with pkgs; [
              openssl
            ];

            meta = with pkgs.lib; {
              description = "Development CLI tool for kueue-operator";
              license = licenses.asl20;
              platforms = platforms.unix;
            };
          };

          # Static musl build for x86_64 Linux
          musl-static = pkgsMusl.pkgsStatic.rustPlatform.buildRustPackage {
            pname = "kueue-dev";
            version = "0.5.6";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgsMusl.pkgsStatic; [
              pkg-config
            ];

            buildInputs = with pkgsMusl.pkgsStatic; [
              openssl
            ];

            # Ensure static linking
            CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";

            meta = with pkgs.lib; {
              description = "Development CLI tool for kueue-operator (statically linked with musl)";
              license = licenses.asl20;
              platforms = [ "x86_64-linux" ];
            };
          };
        };

        devShells.default = with pkgs; mkShell {
          buildInputs = [
            openssl
            pkg-config
            eza
            fd
            rust-bin.stable.latest.default
            mdbook
            operator-sdk
          ];

          shellHook = ''
            alias ls=eza
            alias find=fd
          '';
        };
      }
    );
}
