{ lib, stdenvNoCC, extensionSrc }:

stdenvNoCC.mkDerivation {
  pname = "tiler-gnome-extension";
  version = "0.1.0";

  src = extensionSrc;

  dontBuild = true;
  dontConfigure = true;

  uuid = "tiler@gnome-extensions";

  installPhase = ''
    runHook preInstall

    extDir="$out/share/gnome-shell/extensions/$uuid"
    mkdir -p "$extDir"
    cp metadata.json "$extDir/"
    cp extension.js "$extDir/"
    cp dbus.js "$extDir/"
    cp menu.js "$extDir/"
    cp dbus-interface.xml "$extDir/"

    runHook postInstall
  '';

  meta = {
    description = "Tiler — GNOME Shell extension for tiling window management via D-Bus";
    license = lib.licenses.gpl3Plus;
    platforms = lib.platforms.linux;
  };
}
