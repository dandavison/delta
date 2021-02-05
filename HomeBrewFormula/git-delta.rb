class GitDelta < Formula
  version "0.6.0"
  desc "A viewer for git and diff output"
  homepage "https://github.com/dandavison/delta"

  disable! because: "it is now in homebrew core. Please reinstall it as follows:\nbrew untap dandavison/delta\nbrew install git-delta\n"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "4124b38e7bbd73259c8119e9be5cbe8d4de9dfcbd9fac8f375466b185536d8b3"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "20b59a1c0b74c0f0c0d3c72d7ddbfa857f5c49dd281b84d080263aa5d9da496a"
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
