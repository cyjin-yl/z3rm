#!/usr/bin/env sh
set -eu

# Downloads a tarball from https://zed.dev/releases and unpacks it
# into ~/.local/. If you'd prefer to do this manually, instructions are at
# https://zed.dev/docs/linux.

main() {
    platform="$(uname -s)"
    arch="$(uname -m)"
    channel="${Z3RM_CHANNEL:-stable}"
    Z3RM_VERSION="${Z3RM_VERSION:-latest}"
    # Use TMPDIR if available (for environments with non-standard temp directories)
    if [ -n "${TMPDIR:-}" ] && [ -d "${TMPDIR}" ]; then
        temp="$(mktemp -d "$TMPDIR/z3rm-XXXXXX")"
    else
        temp="$(mktemp -d "/tmp/z3rm-XXXXXX")"
    fi

    if [ "$platform" = "Darwin" ]; then
        platform="macos"
    elif [ "$platform" = "Linux" ]; then
        platform="linux"
    else
        echo "Unsupported platform $platform"
        exit 1
    fi

    case "$platform-$arch" in
        macos-arm64* | linux-arm64* | linux-armhf | linux-aarch64)
            arch="aarch64"
            ;;
        macos-x86* | linux-x86* | linux-i686*)
            arch="x86_64"
            ;;
        *)
            echo "Unsupported platform or architecture"
            exit 1
            ;;
    esac

    if command -v curl >/dev/null 2>&1; then
        curl () {
            command curl -fL "$@"
        }
    elif command -v wget >/dev/null 2>&1; then
        curl () {
            wget -O- "$@"
        }
    else
        echo "Could not find 'curl' or 'wget' in your path"
        exit 1
    fi

    "$platform" "$@"

    if [ "$(command -v zed)" = "$HOME/.local/bin/z3rm" ]; then
        echo "Z3rm has been installed. Run with 'z3rm'"
    else
        echo "To run Zed from your terminal, you must add ~/.local/bin to your PATH"
        echo "Run:"

        case "$SHELL" in
            *zsh)
                echo "   echo 'export PATH=\$HOME/.local/bin:\$PATH' >> ~/.zshrc"
                echo "   source ~/.zshrc"
                ;;
            *fish)
                echo "   fish_add_path -U $HOME/.local/bin"
                ;;
            *)
                echo "   echo 'export PATH=\$HOME/.local/bin:\$PATH' >> ~/.bashrc"
                echo "   source ~/.bashrc"
                ;;
        esac

        echo "To run Zed now, '~/.local/bin/z3rm'"
    fi
}

linux() {
    if [ -n "${Z3RM_BUNDLE_PATH:-}" ]; then
        cp "$Z3RM_BUNDLE_PATH" "$temp/z3rm-linux-$arch.tar.gz"
    else
        echo "Downloading Zed version: $Z3RM_VERSION"
        curl "https://cloud.zed.dev/releases/$channel/$Z3RM_VERSION/download?asset=z3rm&arch=$arch&os=linux&source=install.sh" > "$temp/z3rm-linux-$arch.tar.gz"
    fi

    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    appid=""
    case "$channel" in
      stable)
        appid="dev.z3rm.Z3rm"
        ;;
      nightly)
        appid="dev.z3rm.Z3rm-Nightly"
        ;;
      preview)
        appid="dev.z3rm.Z3rm-Preview"
        ;;
      dev)
        appid="dev.z3rm.Z3rm-Dev"
        ;;
      *)
        echo "Unknown release channel: ${channel}. Using stable app ID."
        appid="dev.z3rm.Z3rm"
        ;;
    esac

    # Unpack
    rm -rf "$HOME/.local/z3rm$suffix.app"
    mkdir -p "$HOME/.local/z3rm$suffix.app"
    tar -xzf "$temp/z3rm-linux-$arch.tar.gz" -C "$HOME/.local/"

    # Setup ~/.local directories
    mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"

    # Link the binary
    if [ -f "$HOME/.local/z3rm$suffix.app/bin/z3rm" ]; then
        ln -sf "$HOME/.local/z3rm$suffix.app/bin/z3rm" "$HOME/.local/bin/z3rm"
    else
        # support for versions before 0.139.x.
        ln -sf "$HOME/.local/z3rm$suffix.app/bin/cli" "$HOME/.local/bin/z3rm"
    fi

    # Copy .desktop file
    desktop_file_path="$HOME/.local/share/applications/${appid}.desktop"
    src_dir="$HOME/.local/zed$suffix.app/share/applications"
    if [ -f "$src_dir/${appid}.desktop" ]; then
        cp "$src_dir/${appid}.desktop" "${desktop_file_path}"
    else
        # Fallback for older tarballs
        cp "$src_dir/z3rm$suffix.desktop" "${desktop_file_path}"
    fi
    sed -i "s|Icon=z3rm|Icon=$HOME/.local/z3rm$suffix.app/share/icons/hicolor/512x512/apps/z3rm.png|g" "${desktop_file_path}"
    sed -i "s|Exec=z3rm|Exec=$HOME/.local/z3rm$suffix.app/bin/z3rm|g" "${desktop_file_path}"
}

macos() {
    echo "Downloading Zed version: $Z3RM_VERSION"
    curl "https://cloud.zed.dev/releases/$channel/$Z3RM_VERSION/download?asset=z3rm&os=macos&arch=$arch&source=install.sh" > "$temp/Z3rm-$arch.dmg"
    hdiutil attach -quiet "$temp/Z3rm-$arch.dmg" -mountpoint "$temp/mount"
    app="$(cd "$temp/mount/"; echo *.app)"
    echo "Installing $app"
    if [ -d "/Applications/$app" ]; then
        echo "Removing existing $app"
        rm -rf "/Applications/$app"
    fi
    ditto "$temp/mount/$app" "/Applications/$app"
    hdiutil detach -quiet "$temp/mount"

    mkdir -p "$HOME/.local/bin"
    # Link the binary
    ln -sf "/Applications/$app/Contents/MacOS/cli" "$HOME/.local/bin/z3rm"
}

main "$@"
