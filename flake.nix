{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, rust-overlay }:
    let
      inherit (nixpkgs) lib;
      eachSystem = lib.genAttrs lib.systems.flakeExposed;
    in {
      devShells = eachSystem (system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.mkShell {
          nativeBuildInputs = [
            (rust-overlay.packages.${system}.rust-nightly_2024-01-01.override {
              extensions = ["rust-src"];
            })
          ];
        };
      });
    };
}
