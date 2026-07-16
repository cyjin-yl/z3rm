#!/usr/bin/env sh
set -eu

# Uninstalls Zed that was installed using the install.sh script

check_remaining_installations() {
    platform="$(uname -s)"
    if [ "$platform" = "Darwin" ]; then
        # Check for any Zed variants in /Applications
        remaining=$(ls -d /Applications/Zed*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    else
        # Check for any Zed variants in ~/.local
        remaining=$(ls -d "$HOME/.local/z3rm"*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    fi
}

prompt_remove_preferences() {
    printf "Do you want to keep your Zed preferences? [Y/n] "
    read -r response
    case "$response" in
        [nN]|[nN][oO])
            rm -rf "$HOME/.config/z3rm"
            echo "Preferences removed."
            ;;
        *)
            echo "Preferences kept."
            ;;
    esac
}

main() {
    platform="$(uname -s)"
    channel="${Z3RM_CHANNEL:-stable}"

    if [ "$platform" = "Darwin" ]; then
        platform="macos"
    elif [ "$platform" = "Linux" ]; then
        platform="linux"
    else
        echo "Unsupported platform $platform"
        exit 1
    fi

    "$platform"

    echo "Zed has been uninstalled"
}

linux() {
    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    appid=""
    db_suffix="stable"
    case "$channel" in
      stable)
        appid="dev.z3rm.Z3rm"
        db_suffix="stable"
        ;;
      nightly)
        appid="dev.z3rm.Z3rm-Nightly"
        db_suffix="nightly"
        ;;
      preview)
        appid="dev.z3rm.Z3rm-Preview"
        db_suffix="preview"
        ;;
      dev)
        appid="dev.z3rm.Z3rm-Dev"
        db_suffix="dev"
        ;;
      *)
        echo "Unknown release channel: ${channel}. Using stable app ID."
        appid="dev.z3rm.Z3rm"
        db_suffix="stable"
        ;;
    esac

    # Remove the app directory
    rm -rf "$HOME/.local/z3rm$suffix.app"

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/z3rm"

    # Remove the .desktop file
    rm -f "$HOME/.local/share/applications/${appid}.desktop"

    # Remove the database directory for this channel
    rm -rf "$HOME/.local/share/z3rm/db/0-$db_suffix"

    # Remove socket file
    rm -f "$HOME/.local/share/z3rm/z3rm-$db_suffix.sock"

    # Remove the entire Zed directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/.local/share/z3rm"
        prompt_remove_preferences
    fi

    rm -rf $HOME/.z3rm_server
}

macos() {
    app="Z3rm.app"
    db_suffix="stable"
    app_id="dev.z3rm.Z3rm"
    case "$channel" in
      nightly)
        app="Z3rm Nightly.app"
        db_suffix="nightly"
        app_id="dev.z3rm.Z3rm-Nightly"
        ;;
      preview)
        app="Z3rm Preview.app"
        db_suffix="preview"
        app_id="dev.z3rm.Z3rm-Preview"
        ;;
      dev)
        app="Z3rm Dev.app"
        db_suffix="dev"
        app_id="dev.z3rm.Z3rm-Dev"
        ;;
    esac

    # Remove the app bundle
    if [ -d "/Applications/$app" ]; then
        rm -rf "/Applications/$app"
    fi

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/z3rm"

    # Remove the database directory for this channel
    rm -rf "$HOME/Library/Application Support/Zed/db/0-$db_suffix"

    # Remove app-specific files and directories
    rm -rf "$HOME/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.ApplicationRecentDocuments/$app_id.sfl"*
    rm -rf "$HOME/Library/Caches/$app_id"
    rm -rf "$HOME/Library/HTTPStorages/$app_id"
    rm -rf "$HOME/Library/Preferences/$app_id.plist"
    rm -rf "$HOME/Library/Saved Application State/$app_id.savedState"

    # Remove the entire Zed directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/Library/Application Support/Zed"
        rm -rf "$HOME/Library/Logs/Zed"

        prompt_remove_preferences
    fi

    rm -rf $HOME/.z3rm_server
}

main "$@"
