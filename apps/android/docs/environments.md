# Environments

The project currently has the mechanism for environment configuration, but the environments are still minimal.

Android reads the API base URL from `BuildConfig.API_BASE_URL`, exposed to shared code through:

```kotlin
expect val apiBaseUrl: String
```

Current Android config in `composeApp/build.gradle.kts`:

```kotlin
debug {
    buildConfigField("String", "API_BASE_URL", "\"http://localhost:8080\"")
}
release {
    buildConfigField("String", "API_BASE_URL", "\"http://localhost:8080\"")
}
```

That means `debug` and `release` currently point to the same local URL. Before shipping, split this into real environments.

## Recommended Workflow

Use separate environments for different kinds of work:

- `local`: real local Rust server at `http://localhost:8080`
- `mock`: fake in-app API/repositories for UI work
- `staging`: deployed non-production server, safe test accounts
- `prod`: production server, real accounts only

Daily development should usually use `local` or `mock`, not production.

## Recommended Android Flavor Shape

Add one flavor dimension:

```kotlin
flavorDimensions += "environment"

productFlavors {
    create("local") {
        dimension = "environment"
        buildConfigField("String", "API_BASE_URL", "\"http://localhost:8080\"")
    }

    create("staging") {
        dimension = "environment"
        buildConfigField("String", "API_BASE_URL", "\"https://api-staging.example.com\"")
    }

    create("prod") {
        dimension = "environment"
        buildConfigField("String", "API_BASE_URL", "\"https://api.example.com\"")
    }
}
```

Then build/install specific variants:

```bash
./gradlew :composeApp:installLocalDebug
./gradlew :composeApp:installStagingDebug
./gradlew :composeApp:assembleProdRelease
```

## Test Accounts

For local/staging servers, prefer seeded test accounts over real user accounts:

- `verified@example.com`
- `unverified@example.com`
- `locked@example.com`
- `reset@example.com`

Keep verification and password-reset tokens accessible through a development mailbox, server logs, or a non-production-only mailbox endpoint.

## Production Rules

- Production builds should use HTTPS only.
- Production should not point to `localhost`, a LAN IP, or a staging host.
- Do not use real production accounts for routine UI development.
- Do not expose development mailboxes or token logs in production.
