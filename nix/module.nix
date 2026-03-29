{ self }:

{ config, lib, pkgs, ... }:

let
  cfg = config.services.tiler;
  tomlFormat = pkgs.formats.toml { };

  # Always generate config — merge user settings with defaults
  effectiveSettings = {
    stack_screen_position = "left";
  } // cfg.settings;

  configFile = tomlFormat.generate "tiler-config.toml" effectiveSettings;
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

    # Always generate config file with defaults
    environment.etc."tiler/config.toml".source = configFile;

    # systemd user service for the tiler daemon
    systemd.user.services.tiler = {
      description = "Tiler — tiling window manager daemon for GNOME";
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" "tiler-keybinding.service" ];
      wantedBy = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/tiler daemon";
        Restart = "on-failure";
        RestartSec = 2;
      };
    };

    # Register tiler in the user's custom-keybindings list.
    # programs.dconf.profiles provides system *defaults* which are
    # overridden when the user-level dconf already carries a value for
    # custom-keybindings (e.g. from GNOME Settings).  This oneshot
    # writes directly to the user database so the entry is always
    # present regardless of existing user settings.
    systemd.user.services.tiler-keybinding = {
      description = "Register tiler in GNOME custom-keybindings";
      wantedBy = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = let
          tilerPath = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/tiler/";
          dconfKey = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings";
          dconf = "${pkgs.dconf}/bin/dconf";
        in pkgs.writeShellScript "tiler-register-keybinding" ''
          current=$(${dconf} read "${dconfKey}" 2>/dev/null)

          # Ensure tiler path is in the custom-keybindings list
          if [[ "$current" != *"${tilerPath}"* ]]; then
            if [[ -z "$current" ]] || [[ "$current" == "@as []" ]]; then
              ${dconf} write "${dconfKey}" "['${tilerPath}']"
            else
              ${dconf} write "${dconfKey}" "''${current%]}, '${tilerPath}']"
            fi
          fi

          # Always write keybinding properties — after a flake update the
          # nix store path changes, so the command must be refreshed.
          ${dconf} write "${tilerPath}name" "'Tiler Menu'"
          ${dconf} write "${tilerPath}command" "'${cfg.package}/bin/tiler menu'"
          ${dconf} write "${tilerPath}binding" "'${cfg.keybinding}'"
        '';
      };
    };

    # Enable the GNOME Shell extension and configure keybinding via dconf
    programs.dconf.enable = true;
    programs.dconf.profiles.user.databases = [{
      settings = {
        "org/gnome/shell" = {
          enabled-extensions = [ "tiler@gnome-extensions" ];
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
