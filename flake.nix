{
  description = "Tiling window manager for GNOME on Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, crane }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      craneLib = crane.mkLib pkgs;

      muslTarget = "x86_64-unknown-linux-musl";
      muslRustFlags = "-C target-feature=+crt-static";

      tiler = import ./nix/package.nix {
        inherit craneLib pkgs muslTarget muslRustFlags;
      };
    in
    {
      packages.${system}.default = tiler;

      devShells.${system}.default = craneLib.devShell {
        packages = with pkgs; [
          clippy
          rustfmt
        ];

        CARGO_BUILD_TARGET = muslTarget;
        CARGO_BUILD_RUSTFLAGS = muslRustFlags;
      };
    };
}
