{
  description = "devshell for ESP32C3";

  inputs = {
    idf.url = "github:mirrexagon/nixpkgs-esp-dev";
  };

  outputs = { idf, system }: idf."${system}".devShells.esp32c3-idf;
}
