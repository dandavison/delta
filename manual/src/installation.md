# Installation

You can download an executable for your system:
[Linux (glibc)](https://github.com/dandavison/delta/releases/download/0.14.0/delta-0.14.0-x86_64-unknown-linux-gnu.tar.gz)
|
[Linux (musl)](https://github.com/dandavison/delta/releases/download/0.14.0/delta-0.14.0-x86_64-unknown-linux-musl.tar.gz)
|
[MacOS](https://github.com/dandavison/delta/releases/download/0.14.0/delta-0.14.0-x86_64-apple-darwin.tar.gz)
|
[Windows](https://github.com/dandavison/delta/releases/download/0.14.0/delta-0.14.0-x86_64-pc-windows-msvc.zip)
|
[All](https://github.com/dandavison/delta/releases)

Alternatively you can install delta using a package manager: see [repology.org/git-delta](https://repology.org/project/git-delta/versions).

Note that the package is often called `git-delta`, but the executable installed is called `delta`. Here is a quick summary for selected package managers:

<table>
  <tr>
    <td><a href="https://archlinux.org/packages/community/x86_64/git-delta/">Arch Linux</a></td>
    <td><code>pacman -S git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://crates.io/crates/git-delta">Cargo</a></td>
    <td><code>cargo install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://src.fedoraproject.org/rpms/rust-git-delta">Fedora</a></td>
    <td><code>dnf install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://pkgs.org/download/git-delta">FreeBSD</a></td>
    <td><code>pkg install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://packages.gentoo.org/packages/dev-util/git-delta">Gentoo</a></td>
    <td><code>emerge dev-util/git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://formulae.brew.sh/formula/git-delta">Homebrew</a></td>
    <td><code>brew install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://ports.macports.org/port/git-delta/summary">MacPorts</a></td>
    <td><code>port install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://search.nixos.org/packages?show=delta&query=delta">Nix</a></td>
    <td><code>nix-env -iA nixpkgs.delta</code>
  </tr>
  <tr>
    <td><a href="https://cvsweb.openbsd.org/ports/textproc/delta/">OpenBSD</a></td>
    <td><code>pkg_add delta</code></td>
  </tr>
  <tr>
    <td><a href="https://software.opensuse.org/package/git-delta">openSUSE Tumbleweed</a></td>
    <td><code>zypper install git-delta</code>
  </tr>
  <tr>
    <td><a href="https://github.com/void-linux/void-packages/tree/master/srcpkgs/delta">Void Linux</a></td>
    <td><code>xbps-install -S delta</code>
  </tr>
  <tr>
    <td>Windows (<a href="https://chocolatey.org/packages/delta">Chocolatey</a>)</td>
    <td><code>choco install delta</code></td>
  </tr>
  <tr>
    <td>Windows (<a href="https://scoop.sh/">Scoop</a>)</td>
    <td><code>scoop install delta</code></td>
  </tr>
  <tr>
    <td>Debian / Ubuntu</td>
    <td>
      <code>dpkg -i file.deb</code>
      <br>
      .deb files are on the <a href="https://github.com/dandavison/delta/releases">releases</a> page.
      <br>
      <sup>If you are using Ubuntu <= 19.10 or are mixing apt sources, please read <a href="https://github.com/dandavison/delta/issues/504">#504</a>.</sup>
    </td>
  </tr>
</table>

Users of older MacOS versions (e.g. 10.11 El Capitan) should install using Homebrew, Cargo, or MacPorts: the binaries on the release page will not work.

Behind the scenes, delta uses [`less`](https://www.greenwoodsoftware.com/less/) for paging.
It's important to have a reasonably recent version of less installed.
On MacOS, install `less` from Homebrew. For Windows, see [Using Delta on Windows](./using-delta-on-windows.md).
