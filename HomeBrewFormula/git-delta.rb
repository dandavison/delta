class GitDelta < Formula
  version "0.0.12"
  desc "A syntax-highlighting pager for git"
  homepage "https://github.com/dandavison/delta"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "4e60b6b2aca2397030809c96e4bfc6b467acbdb07c9c93e8c5bc9b1c7503f8e4"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "471a8ccae151f5b892c9b421ed764ec969aa4ffef2f1a61a5ba49a54fb712d80"
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
