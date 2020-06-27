class GitDelta < Formula
  version "0.1.1"
  desc "A syntax-highlighting pager for git"
  homepage "https://github.com/dandavison/delta"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "f7278c13b7d9cb1cc2878d4820478fc3513b8f6f690da62335ae889183170850"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "8980463026ccb2f72c1be40def19a15d3502d7f4d6c8c330f9d0f312ae37aa6e"
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
