{
  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
  };

  inputs.fenix.url = "github:nix-community/fenix";
  inputs.fenix.inputs = { nixpkgs.follows = "nixpkgs"; };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = { self, nixpkgs, devenv, systems, ... } @ inputs:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
    in
      {
      packages = forEachSystem (system: 
        let
          pkgs = nixpkgs.legacyPackages.${system};
          fenixPkgs = inputs.fenix.packages.${system};
          rustToolchain = fenixPkgs.latest.toolchain;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
        in
        {
          devenv-up = self.devShells.${system}.default.config.procfileScript;
          
          default = rustPlatform.buildRustPackage {
            pname = "markdown-oxide";
            version = "0.24.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            meta = with pkgs.lib; {
              description = "Markdown LSP server inspired by Obsidian";
              homepage = "https://github.com/Feel-ix-343/markdown-oxide";
              license = licenses.asl20;
              maintainers = [ ];
              mainProgram = "markdown-oxide";
            };
          };
        });

      devShells = forEachSystem
        (system:
          let
            pkgs = nixpkgs.legacyPackages.${system};
          in
            {
            default = devenv.lib.mkShell {
              inherit inputs pkgs;
              modules = [
                {
                  # https://devenv.sh/reference/options/
                  packages = [ pkgs.hello ];

                  languages.rust.enable = true;
                  languages.rust.channel = "stable";
                }
              ];
            };
          });
    };
}
