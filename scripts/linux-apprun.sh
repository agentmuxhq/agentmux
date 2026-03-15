#!/usr/bin/env bash
# AppImage AppRun script for AgentMux on Linux.
# Replaces linuxdeploy's default AppRun which forces GDK_BACKEND=x11 and
# does not set WEBKIT_DISABLE_DMABUF_RENDERER=1.
#
# Icon / desktop registration:
#   On Wayland, GNOME matches windows to .desktop files by xdg_toplevel.app_id,
#   which equals the GTK application-id (= the tauri identifier baked into
#   usr/share/agentmux/appid at build time).  We register both:
#     - agentmux.desktop        (X11: matched via StartupWMClass=agentmux)
#     - ${app_id}.desktop       (Wayland: matched by app_id filename)
set -e
this_dir="$(readlink -f "$(dirname "$0")")"
export APPDIR="$this_dir"
export WEBKIT_DISABLE_DMABUF_RENDERER=1
# Use native Wayland when available; fall back to X11 for pure X11 sessions.
if [ -n "$WAYLAND_DISPLAY" ]; then
    export GDK_BACKEND=wayland
else
    export GDK_BACKEND=x11
fi
export XMODIFIERS=""
export GTK_IM_MODULE=gtk-im-context-simple
if [ -n "$APPIMAGE" ]; then
    _icon_dir="$HOME/.local/share/icons/hicolor/256x256/apps"
    _apps_dir="$HOME/.local/share/applications"
    mkdir -p "$_icon_dir" "$_apps_dir"
    cp -f "$this_dir/AgentMux.png" "$_icon_dir/agentmux.png"
    # Read the GTK app-id embedded at build time.
    _appid=$(cat "$this_dir/usr/share/agentmux/appid" 2>/dev/null || echo "agentmux")
    _cur=$(grep -m1 "^Exec=" "$_apps_dir/agentmux.desktop" 2>/dev/null | cut -d= -f2- || true)
    if [ "$_cur" != "$APPIMAGE" ]; then
        _content=$(sed "s|^Exec=.*|Exec=$APPIMAGE|" "$this_dir/AgentMux.desktop")
        printf '%s\n' "$_content" > "$_apps_dir/agentmux.desktop"
        printf '%s\n' "$_content" > "$_apps_dir/${_appid}.desktop"
        # Ensure hicolor has an index.theme so gtk-update-icon-cache succeeds
        if [ ! -f "$HOME/.local/share/icons/hicolor/index.theme" ]; then
            cp /usr/share/icons/hicolor/index.theme "$HOME/.local/share/icons/hicolor/index.theme" 2>/dev/null || true
        fi
        update-desktop-database "$_apps_dir" 2>/dev/null || true
        gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
    fi
fi
exec "$this_dir/usr/bin/agentmux" "$@"
