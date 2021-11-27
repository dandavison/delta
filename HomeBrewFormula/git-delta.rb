class GitDelta < Formula
  version "0.10.2"
  desc "A viewer for git and diff output"
  homepage "https://github.com/dandavison/delta"

  disable! because: "it is now in homebrew core. Please reinstall it as follows:\nbrew untap dandavison/delta\nbrew install git-delta\n"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0d7ccd86c1bd25cb9a229a31ee7c3d2db05b9e7909cd76a6c1f9188b8a146189"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "d4c666bd5bda9fd7cd08fde6f62c94afd094afaf2bca936e8985d599b7703446"
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
