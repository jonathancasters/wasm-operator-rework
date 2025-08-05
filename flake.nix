{
  description = "virtual environments";

  inputs.devshell.url = "github:numtide/devshell";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };

  outputs =
    {
      self,
      flake-utils,
      devshell,
      nixpkgs,
      nixpkgs-unstable,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (system: {
      devShells.default =
        let
          unstable-pkgs = import nixpkgs-unstable { inherit system; };

          tinygo-overlay = self: super: {
            tinygo = unstable-pkgs.tinygo;
          };

          pkgs = import nixpkgs {
            inherit system;

            overlays = [ devshell.overlays.default tinygo-overlay ];
          };
        in
        pkgs.devshell.mkShell { 
          imports = [ (pkgs.devshell.importTOML ./devshell.toml) ];
        };
    });
}
