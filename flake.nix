{
  description = "ESP32-C3 firmware + host shenanigans";

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        embeddedTarget = "riscv32imc-unknown-none-elf";

        # ── firmware: cross rustPlatform targeting the ESP32-C3 RISC-V core ──
        firmwareRustPlatform =
          let
            pkgsCross = import nixpkgs {
              inherit system overlays;
              crossSystem = {
                inherit system;
                rust.rustcTarget = embeddedTarget;
              };
            };
          in
          pkgsCross.makeRustPlatform {
            rustc = toolchain;
            cargo = toolchain;
          };

        firmware = firmwareRustPlatform.buildRustPackage {
          pname = "firmware";
          version = "0.0.1";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          cargoBuildFlags = [
            "--package"
            "firmware"
          ];
          doCheck = false;

          RUSTFLAGS = [
            "--cfg"
            "portable_atomic_unsafe_assume_single_core"
            # Link script injected by esp-hal for memory layout.
            "-C"
            "link-arg=-Tlinkall.x"
            "-C"
            "linker=rust-lld"
          ];
        };

        hostRustPlatform = pkgs.makeRustPlatform {
          rustc = toolchain;
          cargo = toolchain;
        };

        host = hostRustPlatform.buildRustPackage {
          pname = "host";
          version = "0.0.1";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "--package"
            "host"
          ];
          cargoTestFlags = [
            "--package"
            "host"
            "--package"
            "shared"
          ];
        };

        flash = pkgs.writeShellApplication {
          name = "flash";
          runtimeInputs = [ pkgs.espflash ];
          text = ''
            espflash flash --monitor "${firmware}/bin/firmware"
          '';
        };

      in
      {
        packages = {
          inherit firmware host flash;
          default = host;
        };

        apps.flash = {
          type = "app";
          program = "${flash}/bin/flash";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            toolchain
            pkgs.espflash
            pkgs.cargo-binutils
            pkgs.cargo-edit
          ];
        };
      }
    );

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };
}
