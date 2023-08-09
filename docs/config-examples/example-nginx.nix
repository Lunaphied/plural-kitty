# This file contains an example Nginx config for Synapse with Plural Kitty
# Sections containing Plural Kitty specific config are marked with comments begging with PK
{
  services.nginx =
    let
      domain = "example.com";
      clientConfig = {
        "m.homeserver" = {
          base_url = "https://${domain}";
          server_name = "example.com";
        };
      };
      serverConfig."m.server" = "${config.services.matrix-synapse.settings.server_name}:443";
      mkWellKnown = config: ''
        add_header Content-Type application/json;
        add_header Access-Control-Allow-Origin *;
        return 200 '${builtins.toJSON config}';
      '';
      pkProxy = {
        proxyPass = "http://pk";
        extraConfig = ''
          proxy_set_header X-Forwarded-For $remote_addr;
          proxy_set_header X-Forwarded-Proto $scheme;
          proxy_set_header Host $host;

          # Synapse responses may be chunked, which is an HTTP/1.1 feature.
          proxy_http_version 1.1;
        '';
      };
    in
    {
      enable = true;

      # PK Create an upstream pointing to Plural Kitty's proxy, falling back to Synapse
      upstreams.pk.servers = {
        "127.0.0.1:4000" = { }; # Plural Kitty's proxy socket address
        "127.0.0.1:8008" = { # Synapses socket address
          backup = true; # Only fall back to this if Plural Kitty is down
        };
      };

      virtualHosts = {
        "${domain}" = {
          enableACME = true;
          forceSSL = true;

          locations = {
            # Normal Synapse config for Nginx
            "= /.well-known/matrix/server".extraConfig = mkWellKnown serverConfig;
            "= /.well-known/matrix/client".extraConfig = mkWellKnown clientConfig;
            "~ ^(/_matrix|/_synapse)" = {
              proxyPass = "http://[::1]:8008";
              extraConfig = ''
                proxy_set_header X-Forwarded-For $remote_addr;
                proxy_set_header X-Forwarded-Proto $scheme;
                proxy_set_header Host $host;

                # Nginx by default only allows file uploads up to 1M in size
                # Increase client_max_body_size to match max_upload_size defined in homeserver.yaml
                client_max_body_size 50M;

                # Synapse responses may be chunked, which is an HTTP/1.1 feature.
                proxy_http_version 1.1;
              '';
            };

            # PK Redirect the message send endpoint to Plural Kitty
            "~ ^(/_matrix/client/[^/]*/rooms/[^/]*/send)" = {
              proxyPass = "http://pk"; # Proxy to Plural Kitty's Nginx upstream configured above
              extraConfig = ''
                proxy_set_header X-Forwarded-For $remote_addr;
                proxy_set_header X-Forwarded-Proto $scheme;
                proxy_set_header Host $host;

                # Synapse responses may be chunked, which is an HTTP/1.1 feature.
                proxy_http_version 1.1;
                '';
            };
          };
        };
      };
    };
}
