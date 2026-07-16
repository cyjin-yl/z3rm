#!/usr/bin/env sh

if [ "$Z3RM_WSL_DEBUG_INFO" = true ]; then
	set -x
fi

Z3RM_PATH="$(dirname "$(realpath "$0")")"

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
    "$Z3RM_PATH/z3rm.exe" --wsl "$WSL_USER@$WSL_DISTRO_NAME" "$@"
    exit $?
else
    "$Z3RM_PATH/z3rm.exe" "$@"
    exit $?
fi
