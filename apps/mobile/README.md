# Mobile App

Kotlin Multiplatform auth client for Android first, with shared Compose UI and shared domain/data layers. The app targets the backend auth API and defaults to `http://localhost:8080` in debug builds.

## Features

- Register, login, logout, logout-all
- Email verification with pasted token and `kmpstarter://verify#token=...` deep link
- Forgot/reset password with pasted token and `kmpstarter://reset#token=...` deep link
- Home screen with current user state and unverified-email banner
- Profile screen for display name, password change, and account deletion
- Login server-status indicator for API reachability
- Ktor client with JSON, logging, bearer token injection, and refresh-token rotation
- Android secure refresh-token storage through EncryptedSharedPreferences
- Common unit tests for DTOs, network APIs, repositories, validators, and view models

## Requirements

- Android Studio JBR 21 or another JDK 21
- Android SDK installed
- Lab Rust Server running on the development machine at `http://localhost:8080`
- Android device or emulator

## Quick Start

Run tests:

```bash
JAVA_HOME=/opt/android-studio/jbr ./gradlew :composeApp:testDebugUnitTest
```

Build the debug APK:

```bash
JAVA_HOME=/opt/android-studio/jbr ./gradlew :composeApp:assembleDebug
```

Install on a connected Android device:

```bash
adb -s RFCY61HAAYK reverse tcp:8080 tcp:8080
adb -s RFCY61HAAYK install -r -t composeApp/build/outputs/apk/debug/composeApp-debug.apk
adb -s RFCY61HAAYK shell am start -n com.example.kmpstarter/.MainActivity
```

For the full real-device workflow, see [docs/real-device-server.md](docs/real-device-server.md).

## Documentation

- [Real device server testing](docs/real-device-server.md)
- [Environment strategy](docs/environments.md)

## Deep Links

Email verification:

```bash
adb shell am start -a android.intent.action.VIEW -d "kmpstarter://verify#token=YOUR_TOKEN"
```

Password reset:

```bash
adb shell am start -a android.intent.action.VIEW -d "kmpstarter://reset#token=YOUR_TOKEN"
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
