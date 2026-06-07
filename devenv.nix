{ pkgs, lib, ... }: {
  packages = with pkgs; [
    clang
    pkg-config
    openssl
    sqlx-cli
  ];

  dotenv.enable = true;

  languages.rust = {
    enable = true;
    mold.enable = true;
  };

  services.postgres = {
    enable = true;
    listen_addresses = "localhost";
    settings.shared_preload_libraries = "timescaledb";
    initialDatabases = [
        {
          name = "newfm";
          schema = ./migrations/0001_initial.sql;
        }
    ];
    extensions = extensions: [
      extensions.timescaledb
    ];
  };

  services.redis = {
    enable = true;
    port = 6379;
    extraConfig = "requirepass 123";
  };
}