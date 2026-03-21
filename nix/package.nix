{ craneLib, pkgs, muslTarget, muslRustFlags }:

let
  src = craneLib.cleanCargoSource (craneLib.path ../.);
in
craneLib.buildPackage {
  inherit src;

  strictDeps = true;

  CARGO_BUILD_TARGET = muslTarget;
  CARGO_BUILD_RUSTFLAGS = muslRustFlags;

  nativeBuildInputs = with pkgs; [
    musl
  ];
}
