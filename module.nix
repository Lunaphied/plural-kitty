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
        Type = "simple";

        DynamicUser = true;

        StateDirectory = "plural-kitty";
        WorkingDirectory = "/var/lib/plural-kitty";
        RuntimeDirectory = "plural-kitty";
        RuntimeDirectoryMode = "0700";

        ExecStart = "${cfg.package}/bin/plural-kitty ${settingsFormat.generate "config.yaml" cfg.settings}";

        Restart = "on-failure";
        RestartSec = 5;
      };
    };

    environment.systemPackages = [ cfg.package ];
  };
}
