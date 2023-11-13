self: { config, lib, pkgs, ... }:
let
  cfg = config.services.plural-kitty;
  settingsFormat = pkgs.formats.yaml { };
in
{
  options.services.plural-kitty = {
    enable = lib.mkEnableOption (lib.mdDoc "Plural Kitty");

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages."${pkgs.system}".default;
      description = ''
        				Overridable attribute of the plural-kitty's package to use.
        			'';
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "plural-kitty";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "plural-kitty";
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
        ExecStart = "${cfg.package}/bin/plural-kitty ${settingsFormat.generate "config.yaml" cfg.settings}";
        Restart = "always";
        RestartSec = 5;
        User = cfg.user;
        Group = cfg.group;
      };
    };
    environment.systemPackages = [ cfg.package ];
    users.users = lib.optionalAttrs (cfg.user == "plural-kitty") {
      "plural-kitty" = {
        group = "plural-kitty";
        isSystemUser = true;
      };
    };
    users.groups = lib.optionalAttrs (cfg.group == "plural-kitty") {
      "plural-kitty" = { };
    };
  };
}
