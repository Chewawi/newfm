{ pkgs, lib, ... }: {
  packages = with pkgs; [
    clang
    pkg-config
    openssl
    sqlx-cli
    turbo
    biome
    just
  ];

  env = {
    OPENSSL_NO_VENDOR = "1";
    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  };

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
