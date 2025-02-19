{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux"];
      perSystem = {
        lib,
        pkgs,
        self',
        ...
      }: let
        inherit (pkgs) stdenv;

        rust-bin = inputs.rust-overlay.lib.mkRustBin {} pkgs;
        toolchain = rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;

        depsArgs = {
          src = lib.fileset.toSource rec {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources root)
              (lib.fileset.fileFilter (file: file.hasExt "sql") root)
              (lib.fileset.maybeMissing ./assets)
            ];
          };
          nativeBuildInputs = [pkgs.pkg-config];
          buildInputs = lib.flatten [
            pkgs.openssl
            (lib.optionals stdenv.hostPlatform.isLinux [
              pkgs.alsa-lib
              pkgs.libpulseaudio
              pkgs.libxkbcommon
              pkgs.xorg.libxcb
            ])
            (lib.optionals stdenv.hostPlatform.isDarwin [
              pkgs.apple-sdk_15
              (pkgs.darwinMinVersionHook "10.15")
            ])
          ];
          cargoExtraArgs = "--features=muzak/runtime_shaders";
        };

        cargoArtifacts = craneLib.buildDepsOnly depsArgs;
        craneArgs = depsArgs // {inherit cargoArtifacts;};

        mkPkg = {withGLES ? false}:
          craneLib.buildPackage (lib.mergeAttrs craneArgs (lib.optionalAttrs stdenv.hostPlatform.isLinux {
            RUSTFLAGS = lib.optionalString withGLES "--cfg gles";

            nativeBuildInputs = craneArgs.nativeBuildInputs ++ [pkgs.autoPatchelfHook];
            runtimeDependencies = [
              pkgs.wayland
              (
                if withGLES
                then pkgs.libglvnd
                else pkgs.vulkan-loader
              )
            ];
          }));
      in {
        formatter = pkgs.alejandra;
        apps = builtins.mapAttrs (_: drv: {program = drv + /bin/muzak;}) self'.packages;
        packages = lib.mergeAttrs {default = mkPkg {};} (lib.optionalAttrs stdenv.hostPlatform.isLinux {
          vulkan = self'.packages.default;
          gles = mkPkg {withGLES = true;};
        });

        checks = lib.mergeAttrs self'.packages {
          cargoClippy = craneLib.cargoClippy craneArgs;
          cargoTarpaulin = craneLib.cargoTarpaulin craneArgs;
        };

        devShells.default = let
          adapters = lib.flatten [
            (lib.optional stdenv.hostPlatform.isLinux pkgs.stdenvAdapters.useMoldLinker)
          ];
          craneDevShell = craneLib.devShell.override {
            mkShell = pkgs.mkShell.override {
              stdenv = builtins.foldl' (acc: adapter: adapter acc) stdenv adapters;
            };
          };
        in
          craneDevShell {
            inherit (self') checks;
            packages = [pkgs.bacon];
            LD_LIBRARY_PATH = lib.optionalString stdenv.hostPlatform.isLinux (
              lib.makeLibraryPath [
                pkgs.libglvnd
                pkgs.vulkan-loader
                pkgs.wayland
              ]
            );
            shellHook = ''
              rustc -Vv
            '';
          };
      };
    };
}
