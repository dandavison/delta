class GitDelta < Formula
  version "0.15.0"
  desc "A viewer for git and diff output"
  homepage "https://github.com/dandavison/delta"

  disable! because: "it is now in homebrew core. Please reinstall it as follows:\nbrew untap dandavison/delta\nbrew install git-delta\n"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "ca97e1216f30f31ca08e4dafa879e03aa8d0f5761d851c47b2c42295953e045f"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "0887113c9f49ccc93d8afaa5cf0e33b8d95c5d5183a4552c5899b1c36c6ec520"
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
