class GitDelta < Formula
  version "0.0.16"
  desc "A syntax-highlighting pager for git"
  homepage "https://github.com/dandavison/delta"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "64c22b879df94c2e60be555ff65928b01290a43780d7148772cc4e927a6cb7fa"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "eefc06afebd50849f89b64b32ed6955d55bf04b313deea56cbd0863a4335406c"
  end

  conflicts_with "delta"

  def install
    bin.install "delta"
    ohai "To configure git to use delta, run:"
    ohai "git config --global core.pager \"delta --dark\"  # --light for light terminal backgrounds"
  end

  test do
    shell_output "#{bin}/delta --compare-themes"
  end
end
