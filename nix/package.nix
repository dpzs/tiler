{ craneLib, pkgs, muslTarget }:

let
  src = craneLib.cleanCargoSource (craneLib.path ../.);
in
craneLib.buildPackage {
  inherit src;

  strictDeps = true;

  CARGO_BUILD_TARGET = muslTarget;

  # Target-specific rustflags — only applies to the musl target, not build scripts
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.pkgsCross.musl64.stdenv.cc}/bin/x86_64-unknown-linux-musl-cc";

  nativeBuildInputs = [
    pkgs.pkgsCross.musl64.stdenv.cc
  ];

  # musl targets default to static linking, no need for -C target-feature=+crt-static
}
