class GitDelta < Formula
  version "0.0.11"
  desc "A syntax-highlighting pager for git"
  homepage "https://github.com/dandavison/delta"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "ebb67a1ba9bd9a3f73f253066a94e7f892bd2f4c711107cda7b489ef71c8229a"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "3475826e6cd1f7a0d63346ca35e973c27caaaa9594ae9d2abf696287cf0949e2"
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
