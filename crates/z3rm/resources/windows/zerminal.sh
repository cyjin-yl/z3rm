#!/usr/bin/env sh

if [ "$ZERMINAL_WSL_DEBUG_INFO" = true ]; then
	set -x
fi

ZERMINAL_PATH="$(dirname "$(realpath "$0")")"

IN_WSL=false
if [ -n "$WSL_DISTRO_NAME" ]; then
	# $WSL_DISTRO_NAME is available since WSL builds 18362, also for WSL2
	IN_WSL=true
fi

if [ $IN_WSL = true ]; then
    WSL_USER="$USER"
    if [ -z "$WSL_USER" ]; then
        WSL_USER="$USERNAME"
    fi
    "$ZERMINAL_PATH/zerminal.exe" --wsl "$WSL_USER@$WSL_DISTRO_NAME" "$@"
    exit $?
else
    "$ZERMINAL_PATH/zerminal.exe" "$@"
    exit $?
fi
