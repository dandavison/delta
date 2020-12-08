#!/usr/bin/env bash
# Building and packaging for release

set -ex

pack() {
    local tempdir
    local out_dir
    local package_name
    local gcc_prefix

    tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t tmp)
    out_dir=$(pwd)
    package_name="$PROJECT_NAME-${GITHUB_REF/refs\/tags\//}-$TARGET"

    if [[ $TARGET == "arm-unknown-linux-gnueabihf" ]]; then
        gcc_prefix="arm-linux-gnueabihf-"
    elif [[ $TARGET == "aarch64-unknown-linux-gnu" ]]; then
        gcc_prefix="aarch64-linux-gnu-"
    else
        gcc_prefix=""
    fi

    # create a "staging" directory
    mkdir "$tempdir/$package_name"

    # copying the main binary
    cp "target/$TARGET/release/$PROJECT_NAME" "$tempdir/$package_name/"
    if [ "$OS_NAME" != windows-latest ]; then
        "${gcc_prefix}"strip "$tempdir/$package_name/$PROJECT_NAME"
    fi

    # manpage, readme and license
    cp README.md "$tempdir/$package_name"
    cp LICENSE "$tempdir/$package_name"

    # archiving
    pushd "$tempdir"
    if [ "$OS_NAME" = windows-latest ]; then
        7z a "$out_dir/$package_name.zip" "$package_name"/*
    else
        tar czf "$out_dir/$package_name.tar.gz" "$package_name"/*
    fi
    popd
    rm -r "$tempdir"
}

make_deb() {
    local tempdir
    local architecture
    local version
    local dpkgname
    local conflictname
    local gcc_prefix
    local homepage
    local maintainer

    homepage="https://github.com/dandavison/delta"
    maintainer="Dan Davison <dandavison7@gmail.com>"
    copyright_years="2019 - "$(date "+%Y")

    case $TARGET in
        x86_64*)
            architecture=amd64
            gcc_prefix=""
            library_dir=""
            ;;
        i686*)
            architecture=i386
            gcc_prefix=""
            library_dir=""
            ;;
        aarch64*)
            architecture=arm64
            gcc_prefix="aarch64-linux-gnu-"
            library_dir="-l/usr/aarch64-linux-gnu/lib"
            ;;
        arm*hf)
            architecture=armhf
            gcc_prefix="arm-linux-gnueabihf-"
            library_dir="-l/usr/arm-linux-gnueabihf/lib"
            ;;
        *)
            echo "make_deb: skipping target '${TARGET}'" >&2
            return 0
            ;;
    esac
    version=${GITHUB_REF/refs\/tags\//}

    if [[ $TARGET = *musl* ]]; then
      dpkgname=$PACKAGE_NAME-musl
      conflictname=$PROJECT_NAME
    else
      dpkgname=$PACKAGE_NAME
      conflictname=$PROJECT_NAME-musl
    fi

    tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t tmp)

    # copy the main binary
    install -Dm755 "target/$TARGET/release/$PROJECT_NAME" "$tempdir/usr/bin/$PROJECT_NAME"
    "${gcc_prefix}"strip "$tempdir/usr/bin/$PROJECT_NAME"

    # Work out shared library dependencies
    # dpkg-shlibdeps requires debian/control file. Dummy it and clean up
    mkdir "./debian"
    touch "./debian/control"
    depends="$(dpkg-shlibdeps $library_dir -O "$tempdir/usr/bin/$PROJECT_NAME" 2> /dev/null | sed 's/^shlibs:Depends=//')"
    rm -rf "./debian"

    # readme and license
    install -Dm644 README.md "$tempdir/usr/share/doc/$dpkgname/README.md"
    cat > "$tempdir/usr/share/doc/$dpkgname/copyright" <<EOF
Format: http://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: $PROJECT_NAME
Source: $homepage

Files: *
Copyright: $copyright_years $maintainer
License: MIT

License: MIT
 Permission is hereby granted, free of charge, to any
 person obtaining a copy of this software and associated
 documentation files (the "Software"), to deal in the
 Software without restriction, including without
 limitation the rights to use, copy, modify, merge,
 publish, distribute, sublicense, and/or sell copies of
 the Software, and to permit persons to whom the Software
 is furnished to do so, subject to the following
 conditions:
 .
 The above copyright notice and this permission notice
 shall be included in all copies or substantial portions
 of the Software.
 .
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
 ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
 TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
 PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
 SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
 CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
 OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
 IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 DEALINGS IN THE SOFTWARE.
EOF
    chmod 644 "$tempdir/usr/share/doc/$dpkgname/copyright"

    # Control file
    mkdir "$tempdir/DEBIAN"
    cat > "$tempdir/DEBIAN/control" <<EOF
Package: $dpkgname
Version: $version
Section: utils
Priority: optional
Maintainer: Dan Davison <dandavison7@gmail.com>
Architecture: $architecture
Depends: $depends
Provides: $PROJECT_NAME
Conflicts: $conflictname
Description: Syntax highlighter for git.
 Delta provides language syntax-highlighting, within-line insertion/deletion
 detection, and restructured diff output for git on the command line.
EOF

    fakeroot dpkg-deb --build "$tempdir" "${dpkgname}_${version}_${architecture}.deb"
}


main() {
    pack
    if [[ $TARGET = *linux* ]]; then
      make_deb
    fi
}

main
