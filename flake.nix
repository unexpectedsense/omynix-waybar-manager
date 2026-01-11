{
  description = "Waybar Manager - Intelligent waybar manager for multiple monitors and Windows Manager - Niri,
    Hyprland and MangoWc";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        buildInputs = with pkgs; [
          # Rust toolchain
          rustToolchain
          
          # Build dependencies
          pkg-config
          
          # Runtime dependencies for project
          jq
        ];

        nativeBuildInputs = with pkgs; [
          # Development tools
          cargo-watch
          cargo-edit
          cargo-outdated
          bacon
          
          # Format & linting
          rustfmt
          clippy
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          
          shellHook = ''
            echo "🦀 Omynix Waybar Manager - Development environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
            echo "Useful commands:"
            echo "  cargo build --release  - Compile the project"
            echo "  cargo run              - Execute"
            echo "  cargo test             - Run tests"
            echo "  cargo watch -x run     - Development with hot-reload"
            echo "  bacon                  - Continuous checking"
            echo ""
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "omynix-waybar-manager";
          version = "0.1.0";
          
          src = ./.;
          
          cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
          
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ jq ];
          
          meta = with pkgs.lib; {
            description = "Waybar Manager - Intelligent waybar manager for multiple monitors and Windows Manager - Niri,
    Hyprland and MangoWc";
            homepage = "https://https://github.com/unexpectedsense/omynix-waybar-manager";
            license = licenses.mit;
            maintainers = [ ];
          };
        };
      }
    );
}
