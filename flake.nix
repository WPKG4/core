{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {self, ...} @ inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [(import inputs.rust-overlay)];
      };
      toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
      treefmt = inputs.treefmt-nix.lib.evalModule pkgs {
        projectRootFile = "flake.nix";
        programs = {
          alejandra.enable = true;
          deadnix.enable = true;
          statix.enable = true;
          rustfmt.enable = true;
        };
      };
    in {
      packages = {
        default = self.packages.${system}.wpkg4;
        wpkg4 =
          (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          })
          .buildRustPackage {
            name = "wpkg4";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [pkgs.pkg-config];
            buildInputs = [pkgs.openssl];
          };
      };
      devShells.default = pkgs.mkShell {
        inherit (self.packages.${system}.default) buildInputs;
        nativeBuildInputs =
          [toolchain]
          ++ self.packages.${system}.default.nativeBuildInputs;
      };
      formatter = treefmt.config.build.wrapper;
      checks.formatting = treefmt.config.build.check self;
    });
}
