{
  description = "yaaaaaaaaaaaaaaaaaaaaa";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.05";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs @ {self, ...}:
    inputs.flake-utils.lib.eachSystem ["x86_64-linux"] (system: let
      pkgs = import inputs.nixpkgs {
        inherit system;
      };
      unstable = import inputs.nixpkgs-unstable {
        inherit system;
      };

      manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
    in {
      packages.default = unstable.rustPlatform.buildRustPackage {
        pname = manifest.name;
        version = manifest.version;
        cargoLock.lockFile = ./Cargo.lock;
        src = pkgs.lib.cleanSource ./.;

        # - [nix flake rust and pkgconfig](https://discourse.nixos.org/t/nix-and-rust-how-to-use-pkgconfig/17465/3)
        buildInputs = with pkgs; [
          openssl
          xdotool
          wtype
        ];
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];
      };

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs;
          [
            unstable.rust-analyzer
            unstable.rustfmt
            unstable.clippy
          ]
          ++ self.packages."${system}".default.nativeBuildInputs
          ++ self.packages."${system}".default.buildInputs;
        shellHook = ''
          export RUST_BACKTRACE="1"
        '';
      };
    });
}
