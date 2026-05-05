# AppName KMP Auth Starter

Kotlin Multiplatform auth starter for Android first, with shared Compose UI and shared domain/data layers. The app targets the Lab Rust Server auth API at `http://localhost:8080`.

## What Is Included

- Register, login, logout, logout-all
- Email verification with pasted token and `kmpstarter://verify?token=...` deep link
- Forgot/reset password with pasted token and `kmpstarter://reset?token=...` deep link
- Home screen with current user state and unverified-email banner
- Profile screen for display name, password change, and account deletion
- Ktor client with JSON, logging, bearer token injection, and refresh-token rotation
- Android secure refresh-token storage through EncryptedSharedPreferences
- Common unit tests for DTOs, network APIs, repositories, validators, and view models

## Requirements

- Android Studio JBR 21 or another JDK 21
- Android SDK installed
- Lab Rust Server running on the development machine at `http://localhost:8080`
- Android device or emulator

This workspace has been verified with:

```bash
JAVA_HOME=/opt/android-studio/jbr ./gradlew :composeApp:testDebugUnitTest
```

## Run On Android

The debug build uses `http://localhost:8080` as `API_BASE_URL`.

On Android, `localhost` means the phone itself, not your computer. When testing a real phone against a server running on your development machine, use ADB reverse so the phone's `localhost:8080` forwards to the machine's `localhost:8080`.

Start your server first, then confirm it responds from the development machine:

```bash
curl -i http://localhost:8080/
```

It is OK if this returns `404 Not Found`; that still proves the server is reachable. For the auth API, this should return `405 Method Not Allowed` because login is a `POST` endpoint:

```bash
curl -i http://localhost:8080/v1/auth/login
```

Connect the phone and confirm ADB sees it:

```bash
adb devices
```

Forward device port `8080` to the development machine:

```bash
adb -s RFCY61HAAYK reverse tcp:8080 tcp:8080
```

Confirm the reverse mapping is active:

```bash
adb -s RFCY61HAAYK reverse --list
```

Expected output includes:

```text
UsbFfs tcp:8080 tcp:8080
```

Build, install, and launch:

```bash
JAVA_HOME=/opt/android-studio/jbr ./gradlew :composeApp:installDebug
adb -s RFCY61HAAYK shell am start -n com.example.kmpstarter/.MainActivity
```

If Gradle's `installDebug` cannot see the device but `adb devices` does, install the built APK directly:

```bash
JAVA_HOME=/opt/android-studio/jbr ./gradlew :composeApp:assembleDebug
adb -s RFCY61HAAYK install -r -t composeApp/build/outputs/apk/debug/composeApp-debug.apk
adb -s RFCY61HAAYK shell am start -n com.example.kmpstarter/.MainActivity
```

The login screen has a server-status indicator:

- Green dot / `Server online`: the app can reach the configured API host.
- Red dot / `Server offline`: the phone cannot reach `http://localhost:8080`.
- Checking/unknown: the status probe has not completed yet.

If the dot says offline while the server is running, the usual cause is that ADB reverse disappeared. Re-run:

```bash
adb -s RFCY61HAAYK reverse tcp:8080 tcp:8080
adb -s RFCY61HAAYK shell am force-stop com.example.kmpstarter
adb -s RFCY61HAAYK shell am start -n com.example.kmpstarter/.MainActivity
```

ADB reverse can disappear after unplugging the phone, restarting ADB, rebooting the phone, or reconnecting the device.

### Testing Without ADB Reverse

For Wi-Fi or non-USB testing, point the app at your computer's LAN IP instead of `localhost`.

1. Make the server bind to all interfaces, usually `0.0.0.0:8080`, not only `127.0.0.1:8080`.
2. Find your computer's LAN IP, for example `192.168.1.15`.
3. Change `API_BASE_URL` in `composeApp/build.gradle.kts`:

```kotlin
buildConfigField("String", "API_BASE_URL", "\"http://192.168.1.15:8080\"")
```

4. Rebuild and reinstall the app.

Use this only on a trusted local network. For production, use HTTPS and a real domain.

## Deep Links

Email verification:

```bash
adb shell am start -a android.intent.action.VIEW -d "kmpstarter://verify?token=YOUR_TOKEN"
```

Password reset:

```bash
adb shell am start -a android.intent.action.VIEW -d "kmpstarter://reset?token=YOUR_TOKEN"
```

## Manual Smoke Test

1. Register a new account.
2. Open the server mail log or dev mailbox and copy the verification token.
3. Paste the token in the verify screen, or launch the verify deep link.
4. Confirm home shows the email as verified.
5. Log out, then log in with the same credentials.
6. Request a password reset, copy the reset token, and set a new password.
7. Confirm login works with the new password.
8. Edit the display name in Profile.
9. Change password from Profile and confirm the session stays active.
10. Test logout-all and account deletion.

## Project Shape

The app lives in `composeApp`:

- `commonMain`: shared DTOs, Ktor APIs, repositories, validators, navigation, Compose screens
- `androidMain`: activity, application, Android DI module, secure storage, network config
- `iosMain`: framework entry point, Darwin Ktor engine, settings-backed token storage placeholder
- `commonTest`: repository, API, DTO, validator, and view model tests

The package is intentionally generic: `com.example.kmpstarter`. Rename it when you adopt the starter.
