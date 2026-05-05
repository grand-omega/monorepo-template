#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
MOBILE_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
APK="$MOBILE_DIR/composeApp/build/outputs/apk/debug/composeApp-debug.apk"
PACKAGE="com.example.kmpstarter"
ACTIVITY="$PACKAGE/.MainActivity"

device="${ANDROID_SERIAL:-}"
if [[ -z "$device" ]]; then
  mapfile -t devices < <(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')
  case "${#devices[@]}" in
    0)
      echo "No authorized Android device found. Check USB debugging and run: adb devices" >&2
      exit 1
      ;;
    1)
      device="${devices[0]}"
      ;;
    *)
      echo "Multiple Android devices found. Set ANDROID_SERIAL to one of:" >&2
      printf '  %s\n' "${devices[@]}" >&2
      exit 1
      ;;
  esac
fi

adb_for_device=(adb -s "$device")

cd "$MOBILE_DIR"

echo "Using Android device: $device"
echo "Building debug APK..."
if [[ -z "${JAVA_HOME:-}" || ! -x "${JAVA_HOME:-}/bin/java" ]]; then
  export JAVA_HOME="/opt/android-studio/jbr"
fi
./gradlew :composeApp:assembleDebug

echo "Restoring localhost:8080 -> development machine:8080..."
"${adb_for_device[@]}" reverse tcp:8080 tcp:8080 >/dev/null
"${adb_for_device[@]}" reverse --list

echo "Installing APK..."
"${adb_for_device[@]}" install -r -t "$APK"

echo "Launching app..."
"${adb_for_device[@]}" shell am force-stop "$PACKAGE"
"${adb_for_device[@]}" shell am start -n "$ACTIVITY" >/dev/null

echo "Done. Login should show the server as online when the backend is running."
