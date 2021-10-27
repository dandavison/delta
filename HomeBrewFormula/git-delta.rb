class GitDelta < Formula
  version "0.9.1"
  desc "A viewer for git and diff output"
  homepage "https://github.com/dandavison/delta"

  disable! because: "it is now in homebrew core. Please reinstall it as follows:\nbrew untap dandavison/delta\nbrew install git-delta\n"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0019dfc4b32d63c1392aa264aed2253c1e0c2fb09216f8e2cc269bbfb8bb49b5"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "0019dfc4b32d63c1392aa264aed2253c1e0c2fb09216f8e2cc269bbfb8bb49b5"
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
