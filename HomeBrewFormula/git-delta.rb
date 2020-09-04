class GitDelta < Formula
  version "0.4.3"
  desc "A viewer for git and diff output"
  homepage "https://github.com/dandavison/delta"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "c6441563648fcc74085a5b079038fca3a1c9c91b8341d8accc07d345da8fc2c6"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "d596927a95b0dc5b6e4bc40970ad14ff38bac4c23182bae5a303b6bb51f06baa"
  end

  conflicts_with "delta"

  def install
    bin.install "delta"
    ohai "To configure git to use delta, run:"
    ohai "git config --global core.pager \"delta --dark\"  # --light for light terminal backgrounds"
  end

  test do
    shell_output "#{bin}/delta --show-syntax-themes"
  end
end
