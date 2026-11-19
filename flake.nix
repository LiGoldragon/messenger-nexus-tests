{
  description = "The Messenger Nexus research prototype's separate core acceptance tests.";
  inputs.base.url = "github:LiGoldragon/build-base";
  outputs = { self, base }: base.lib.workspaceOutputs { cargoNix = ./Cargo.nix; };
}
