class GitDelta < Formula
  version "0.3.0"
  desc "A viewer for git and diff output"
  homepage "https://github.com/dandavison/delta"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "76779cfcbf5327837432866d58ea68ff37508afdfeed3a00102c82b787897cb4"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "33d8bd17fb396adef16ed6ede309fcc3f11c6e8d3c9446e7082b3049275103fb"
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
