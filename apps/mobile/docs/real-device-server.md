# Real Device Server Testing

Use this workflow when the Android app is installed on a physical phone and the Rust API server is running on your development machine.

## Why ADB Reverse Is Needed

The debug Android build uses:

```text
http://localhost:8080
```

On Android, `localhost` means the phone itself. If your server is running on your computer, the phone cannot reach it unless you either:

- use ADB reverse over USB, or
- configure the app to use your computer's LAN IP.

For day-to-day local development, use the helper script. It rebuilds the app,
restores ADB reverse, reinstalls the APK, and launches the app:

```bash
apps/mobile/scripts/dev-phone.sh
```

ADB reverse can disappear after unplugging the phone, restarting ADB, rebooting
the phone, or reconnecting the device, so rerun this script whenever the login
screen unexpectedly says `Server offline`.

## USB Workflow

Start your server first, then confirm it responds from the development machine:

```bash
curl -i http://localhost:8080/
```

`404 Not Found` is acceptable here. It proves the server is reachable even if `/` is not a real route.

Confirm the auth route exists:

```bash
curl -i http://localhost:8080/v1/auth/login
```

Expected result: `405 Method Not Allowed`, because login is a `POST` endpoint.

Connect the phone and run:

```bash
apps/mobile/scripts/dev-phone.sh
```

The manual commands below are useful when debugging the connection.

Confirm ADB sees the phone:

```bash
adb devices
```

Forward the phone's port `8080` to the development machine:

```bash
adb -s RFCY61HAAYK reverse tcp:8080 tcp:8080
```

Confirm the reverse mapping:

```bash
adb -s RFCY61HAAYK reverse --list
```

Expected output includes:

```text
UsbFfs tcp:8080 tcp:8080
```

Build, install, and launch manually:

```bash
JAVA_HOME=/opt/android-studio/jbr ./gradlew :composeApp:installDebug
adb -s RFCY61HAAYK shell am start -n com.example.kmpstarter/.MainActivity
```

If Gradle cannot install but `adb devices` shows the phone, install the APK directly:

```bash
JAVA_HOME=/opt/android-studio/jbr ./gradlew :composeApp:assembleDebug
adb -s RFCY61HAAYK install -r -t composeApp/build/outputs/apk/debug/composeApp-debug.apk
adb -s RFCY61HAAYK shell am start -n com.example.kmpstarter/.MainActivity
```

## Login Server Status Dot

The login screen checks whether the configured API host is reachable.

- Green dot / `Server online`: the app can reach the API host.
- Red dot / `Server offline`: the phone cannot reach `http://localhost:8080`.
- Checking/unknown: the status probe has not completed yet.

If the dot says offline while the server is running, ADB reverse is usually missing. Re-run:

```bash
adb -s RFCY61HAAYK reverse tcp:8080 tcp:8080
adb -s RFCY61HAAYK shell am force-stop com.example.kmpstarter
adb -s RFCY61HAAYK shell am start -n com.example.kmpstarter/.MainActivity
```

ADB reverse can disappear after unplugging the phone, restarting ADB, rebooting the phone, or reconnecting the device.

## Wi-Fi / LAN Testing

For non-USB testing, point the app at your computer's LAN IP instead of `localhost`.

1. Make the server bind to all interfaces, usually `0.0.0.0:8080`, not only `127.0.0.1:8080`.
2. Find your computer's LAN IP, for example `192.168.1.15`.
3. Change `API_BASE_URL` in `composeApp/build.gradle.kts`:

```kotlin
buildConfigField("String", "API_BASE_URL", "\"http://192.168.1.15:8080\"")
```

4. Rebuild and reinstall the app.

Use this only on a trusted local network. For production, use HTTPS and a real domain.
