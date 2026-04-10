{ lib
, rustPlatform
}:

rustPlatform.buildRustPackage rec {
  pname = "cc-switch";
  version = "5.3.0";

  src = lib.cleanSource ../.;

  cargoRoot = "src-tauri";
  buildAndTestSubdir = "src-tauri";
  cargoLock = {
    lockFile = ../src-tauri/Cargo.lock;
  };

  preCheck = ''
    export HOME="$TMPDIR/home"
    mkdir -p "$HOME"
    export CC_SWITCH_TEST_HOME="$HOME"
    export HOSTNAME="cc-switch-nix"
  '';
  # install_script tests spawn runtime-generated helper scripts with a hard-coded
  # /usr/bin/env shebang, which is unavailable in pure Nix sandboxes.
  checkFlags = [
    "--skip=install_script_requires_force_for_non_tty_overwrite"
    "--skip=install_script_force_overwrites_and_warns_about_shadowed_path"
    "--skip=install_script_supports_linux_glibc_override"
    "--skip=install_script_falls_back_to_glibc_when_musl_download_fails"
  ];

  meta = with lib; {
    description = "CLI manager for Claude Code, Codex, Gemini, OpenCode, and OpenClaw";
    homepage = "https://github.com/saladday/cc-switch-cli";
    license = licenses.mit;
    mainProgram = "cc-switch";
    platforms = platforms.unix;
  };
}
