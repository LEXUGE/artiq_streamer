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

        rustBin = pkgs.rust-bin.stable.latest.default;
        # Unstable rustfmt needed for our formatting options
        rustfmt = pkgs.rust-bin.nightly."2024-06-10".rustfmt;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustBin;
          rustc = rustBin;
        };
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            (lib.hiPrio rustfmt)
            rustBin
            zeromq
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
                rustfmt = rustfmt;
                cargo = rustBin;
              };
              clippy.enable = true;
              clippy.packageOverrides = {
                clippy = rustBin;
                cargo = rustBin;
              };
              cargo-check = {
                enable = true;
                package = rustBin;
              };
            };
          };
        };
        packages.artiq_streamer = rustPlatform.buildRustPackage rec {
          pname = "artiq_streamer";
          version = "git";
          src = ./.;
          buildInputs = [ pkgs.zeromq ];
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };
      }
    );
}
