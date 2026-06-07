{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [ pkgs.qemu pkgs.OVMF ];
  
  # 環境変数として正しいパスをエクスポートする
  shellHook = ''
    export OVMF_CODE="${pkgs.OVMF.fd}/FV/OVMF_CODE.fd"
    export OVMF_VARS="${pkgs.OVMF.fd}/FV/OVMF_VARS.fd"
  '';
}
