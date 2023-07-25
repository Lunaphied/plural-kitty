pkg: { config, lib, pkgs, ... }:

let
  cfg = config.services.plural-kitty;
  settingsFormat = pkgs.formats.yaml { };
in
{
  options.services.plural-kitty = {
    enable = lib.mkEnableOption (lib.mdDoc "Plural Kitty");

    package = lib.mkOption {
      type = lib.types.package;
      default = pkg;
      description = ''
        				Overridable attribute of the plural-kitty's package to use.
        			'';
    };

    logString = lib.mkOption {
      type = lib.types.str;
      default = "warn,plural_kitty=info";
    };

    settings = lib.mkOption {
      type = lib.types.submodule {
        freeformType = settingsFormat.type;
      };
      default = { };
      description = ''
        			'';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.plural-kitty = {
      description = "Plural Kitty";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      environment = {
        RUST_LOG = cfg.logString;
      };
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/plural-kitty ${settingsFormat.generate "comfig.yaml" cfg.settings}";
        Restart = "always";
        User = "plural-kitty";
        Group = "plural-kitty";
      };
    };
    environment.systemPackages = [ cfg.package ];
    users = {
      users."plural-kitty" = {
        group = "plural-kitty";
        isSystemUser = true;
      };
      groups."plural-kitty" = { };
    };
  };
}
