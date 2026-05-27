{
  buildFeatures ? [ ],
  buildNoDefaultFeatures ? false,
  buildPackages,
  fetchFromGitHub,
  installManPages ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellCompletions ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellFiles,
  lib,
  rustPlatform,
  stdenv,
}:

let
  version = "0.0.1";
  emul = stdenv.hostPlatform.emulator buildPackages;
  exe = stdenv.hostPlatform.extensions.executable;

in
rustPlatform.buildRustPackage {
  inherit version buildNoDefaultFeatures buildFeatures;

  pname = "m2m";
  cargoHash = "";

  src = fetchFromGitHub {
    hash = "";
    owner = "pimalaya";
    repo = "m2m";
    rev = "v${version}";
  };

  nativeBuildInputs = [
    installShellFiles
  ];

  doCheck = false;

  postInstall =
    lib.optionalString (lib.hasInfix "wine" emul) ''
      export WINEPREFIX="''${WINEPREFIX:-$(mktemp -d)}"
      mkdir -p $WINEPREFIX
    ''
    + ''
      mkdir -p $out/share/{applications,completions,man}
      ${emul} "$out"/bin/m2m${exe} manuals "$out"/share/man
      ${emul} "$out"/bin/m2m${exe} completions -d "$out"/share/completions bash elvish fish powershell zsh
    ''
    + lib.optionalString installManPages ''
      installManPage "$out"/share/man/*
    ''
    + lib.optionalString installShellCompletions ''
      installShellCompletion --cmd m2m \
        --bash "$out"/share/completions/m2m.bash \
        --fish "$out"/share/completions/m2m.fish \
        --zsh "$out"/share/completions/_m2m
    '';

  meta = {
    description = "CLI to convert filesystem-based mailboxes between each other";
    mainProgram = "m2m";
    homepage = "https://github.com/pimalaya/m2m";
    changelog = "https://github.com/pimalaya/m2m/blob/v${version}/CHANGELOG.md";
    license = lib.licenses.agpl3Only;
    maintainers = with lib.maintainers; [ soywod ];
  };
}
