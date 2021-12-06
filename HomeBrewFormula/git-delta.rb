class GitDelta < Formula
  version "0.11.0"
  desc "A viewer for git and diff output"
  homepage "https://github.com/dandavison/delta"

  disable! because: "it is now in homebrew core. Please reinstall it as follows:\nbrew untap dandavison/delta\nbrew install git-delta\n"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "89913bb086367b00c908ef0f2257ee4eb8b23b6b4df56eb9cfb85d351ee49f0a"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "3a2eb0b8d9446bde4284592c8858e4166228991dec471ad4f379306aaa96fa84"
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
