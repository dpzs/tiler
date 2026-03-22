{
  description = "Tiling window manager for GNOME on Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, crane, rust-overlay }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = [ "x86_64-unknown-linux-musl" ];
      };
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      muslTarget = "x86_64-unknown-linux-musl";

      tiler = import ./nix/package.nix {
        inherit craneLib pkgs muslTarget;
      };

      tiler-gnome-extension = pkgs.callPackage ./nix/gnome-extension.nix {
        extensionSrc = ./extension;
      };
    in
    {
      packages.${system} = {
        default = tiler;
        tiler = tiler;
        gnome-extension = tiler-gnome-extension;
      };

      nixosModules.default = import ./nix/module.nix {
        inherit self;
      };

      devShells.${system}.default = craneLib.devShell {
        packages = with pkgs; [
          clippy
          rustfmt
        ];

        CARGO_BUILD_TARGET = muslTarget;
      };
    };
}
