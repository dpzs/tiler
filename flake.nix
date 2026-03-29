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
          # E2E test dependencies (headless GNOME Shell)
          gnome-shell
          dbus
          dconf
          glib          # gdbus, gsettings
          xterm
        ];

        CARGO_BUILD_TARGET = muslTarget;

        # Merge GSettings schemas from gnome-shell 49 and desktop-schemas so
        # headless gnome-shell finds all required keys (e.g. screen-brightness-up)
        shellHook = let
          gschemaDir = pkgs.runCommand "merged-gschemas" {} ''
            mkdir -p $out/glib-2.0/schemas
            for src in \
              ${pkgs.gnome-shell}/share/gsettings-schemas/*/glib-2.0/schemas \
              ${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/*/glib-2.0/schemas \
              ${pkgs.mutter}/share/gsettings-schemas/*/glib-2.0/schemas; do
              if [ -d "$src" ]; then
                cp -n "$src"/*.xml "$out/glib-2.0/schemas/" 2>/dev/null || true
                cp -n "$src"/*.override "$out/glib-2.0/schemas/" 2>/dev/null || true
              fi
            done
            ${pkgs.glib.dev}/bin/glib-compile-schemas "$out/glib-2.0/schemas"
          '';
        in ''
          export GSETTINGS_SCHEMA_DIR="${gschemaDir}/glib-2.0/schemas"
        '';
      };
    };
}
