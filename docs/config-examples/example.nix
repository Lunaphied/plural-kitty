{
  # Plural Kitty's service config
  services.plural-kitty = {
    enable = true;
    user = "matrix-synapse";
    group = "matrix-synapse";
    settings = {
      listen = "127.0.0.1:4000"; # socket address proxy should listen on
      synapse = {
        host = "http://127.0.0.1:8008"; # socket address of Synapse server for proxy
        # DB login info for synapse database (can be mostly copied from Synapse config)
        db = {
          user = "matrix-synapse";
          host = "/run/postgresql";
          database = "synapse";
        };
      };
      bot = {
        user = "@pk:the-apothecary.club"; # Matrix user for Plural Kitty Bot
        secret_file = "/var/secrets/pk-session.json"; # Location to store PK's session token
        homeserver_url = "http://127.0.0.1:8008"; # socket address of Matrix server bot should connect to (probably same as above)
        state_store = "/var/lib/plural-kitty"; # Filesystem location to store bot's state database
        # DB login info for Plural Kitties database (you must create this database beforehand)
        db = {
          user = "matrix-synapse";
          host = "/run/postgresql";
          database = "plural_kitty";
        };
      };
    };
  };
}
