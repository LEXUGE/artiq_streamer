{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };
  outputs = { self, git-hooks, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            rust
          ] ++ self.checks.${system}.pre-commit-check.enabledPackages;
          inherit (self.checks.${system}.pre-commit-check) shellHook;
        };
        checks = {
          pre-commit-check = git-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              nixpkgs-fmt.enable = true;
              rustfmt.enable = true;
              rustfmt.packageOverrides = {
                rustfmt = rust;
                cargo = rust;
              };
              clippy.enable = true;
              clippy.packageOverrides = {
                clippy = rust;
                cargo = rust;
              };
            };
          };
        };
        packages.artiq_streamer = rustPlatform.buildRustPackage rec {
          pname = "artiq_streamer";
          version = "git";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };
      }
    );
}
