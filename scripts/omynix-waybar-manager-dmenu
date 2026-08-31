#!/usr/bin/env bash
# omynix-waybar-manager dmenu wrapper
#
# Drives the waybar manager from walker's dmenu mode, so it can be launched
# from a keybind/launcher without a terminal. It maps dmenu selections to the
# tool's non-interactive CLI flags:
#
#   launch                               detect monitors, generate configs, launch waybar
#   --set-preferred <monitor>            set the preferred monitor in the omynix config
#   --list-monitors                      list available monitors, one per line
#   version                              print the version
#
# Requires: walker (dmenu mode) and omynix-waybar-manager in PATH.
#
# Bind this script to a key in your compositor, e.g. in Niri:
#   spawn "sh" "-c" "$HOME/.local/bin/omynix-waybar-manager-dmenu"

set -euo pipefail

BIN="omynix-waybar-manager"
DMENU="walker --dmenu"

choice=$(
    printf "%s\n" \
        "Launch Waybar" \
        "Set Preferred Monitor" \
        "List Monitors" \
        "Version" |
        $DMENU
)

case "$choice" in
    "Launch Waybar")
        "$BIN" launch
        ;;
    "Set Preferred Monitor")
        monitor=$("$BIN" --list-monitors | $DMENU)
        if [[ -n "$monitor" ]]; then
            "$BIN" --set-preferred "$monitor"
        fi
        ;;
    "List Monitors")
        "$BIN" --list-monitors
        ;;
    "Version")
        "$BIN" version
        ;;
    *)
        # Esc / ninguna selección: salir sin hacer nada.
        exit 0
        ;;
esac
