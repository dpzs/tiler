{ self }:

{ config, lib, pkgs, ... }:

let
  cfg = config.services.tiler;
  tomlFormat = pkgs.formats.toml { };

  configFile = tomlFormat.generate "tiler-config.toml" cfg.settings;
in
{
  options.services.tiler = {
    enable = lib.mkEnableOption "Tiler — tiling window manager for GNOME on Wayland";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.tiler;
      defaultText = lib.literalExpression "self.packages.\${pkgs.system}.tiler";
      description = "The tiler daemon/CLI package to use.";
    };

    gnomeExtensionPackage = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.gnome-extension;
      defaultText = lib.literalExpression "self.packages.\${pkgs.system}.gnome-extension";
      description = "The tiler GNOME Shell extension package to use.";
    };

    keybinding = lib.mkOption {
      type = lib.types.str;
      default = "<Super>t";
      description = ''
        The global keybinding that triggers the tiler menu.
        Uses GNOME/GTK accelerator syntax (e.g. "<Super>t", "<Ctrl><Alt>t").
      '';
    };

    settings = lib.mkOption {
      type = tomlFormat.type;
      default = { };
      description = ''
        Configuration options for tiler, written as TOML.
        The generated file is placed at /etc/tiler/config.toml and
        the daemon is started with --config pointing to it.
      '';
      example = lib.literalExpression ''
        {
          stack_screen_position = "left";
        }
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    # Install the tiler binary and GNOME Shell extension
    environment.systemPackages = [
      cfg.package
      cfg.gnomeExtensionPackage
    ];

    # Generate TOML config file at /etc/tiler/config.toml
    environment.etc."tiler/config.toml" = lib.mkIf (cfg.settings != { }) {
      source = configFile;
    };

    # systemd user service for the tiler daemon
    systemd.user.services.tiler = {
      description = "Tiler — tiling window manager daemon for GNOME";
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/tiler daemon";
        Restart = "on-failure";
        RestartSec = 2;
      };
    };

    # Configure keybinding via dconf/gsettings
    programs.dconf.enable = true;
    programs.dconf.profiles.user.databases = [{
      settings = {
        "org/gnome/settings-daemon/plugins/media-keys" = {
          custom-keybindings = [ "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/tiler/" ];
        };
        "org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/tiler" = {
          name = "Tiler Menu";
          command = "${cfg.package}/bin/tiler menu";
          binding = cfg.keybinding;
        };
      };
    }];
  };
}
