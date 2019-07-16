class GitDelta < Formula
  version "v0.0.1"
  desc "A syntax-highlighting pager for git"
  homepage "https://github.com/dandavison/delta"

  if OS.mac?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "453b4652f55f86ee3aeb128e7cda3cf6cdc9fdbeac4c69a76e4c96c88a9af99b"
  elsif OS.linux?
      url "https://github.com/dandavison/delta/releases/download/#{version}/delta-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "73ef55027433bb72ae2457428ae1c2807d1f807cfaa111692e58ab4f4e27b8d0"
  end

  conflicts_with "delta"

  def install
    bin.install "delta"
  end
end
