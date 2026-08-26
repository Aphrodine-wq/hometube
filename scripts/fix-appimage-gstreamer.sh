#!/usr/bin/env bash
set -euo pipefail

app_run="${1:-src-tauri/target/release/bundle/appimage/HomeTube.AppDir/AppRun}"

if [[ ! -f "$app_run" ]]; then
  printf 'AppRun was not found: %s\n' "$app_run" >&2
  exit 1
fi

app_dir="$(dirname "$app_run")"
plugin_target="$app_dir/usr/lib/gstreamer-1.0"
host_plugin_dir=""

for candidate in /usr/lib/gstreamer-1.0 /usr/lib64/gstreamer-1.0 /usr/lib/x86_64-linux-gnu/gstreamer-1.0; do
  if [[ -d "$candidate" ]]; then
    host_plugin_dir="$candidate"
    break
  fi
done

if [[ -z "$host_plugin_dir" ]]; then
  printf 'No system GStreamer plugin directory was found.\n' >&2
  exit 1
fi

if [[ ! -d "$plugin_target" ]]; then
  mkdir -p "$(dirname "$plugin_target")"
  cp -a "$host_plugin_dir" "$plugin_target"
  printf 'Bundled GStreamer plugins from %s\n' "$host_plugin_dir"
fi

if grep -q "HOMETUBE_GSTREAMER_HOST_PATH" "$app_run"; then
  printf 'GStreamer host-path fix is already present in %s\n' "$app_run"
  exit 0
fi

sed -i '/^exec /i\
# HOMETUBE_GSTREAMER_HOST_PATH: linuxdeploy otherwise hides host media plugins.\
for hometube_gst_dir in /usr/lib/gstreamer-1.0 /usr/lib64/gstreamer-1.0 /usr/lib/x86_64-linux-gnu/gstreamer-1.0; do\
  if [[ -d "$hometube_gst_dir" ]]; then\
    export GST_PLUGIN_SYSTEM_PATH_1_0="${GST_PLUGIN_SYSTEM_PATH_1_0:-$hometube_gst_dir}"\
    break\
  fi\
done\
' "$app_run"

printf 'Added host GStreamer discovery to %s\n' "$app_run"
