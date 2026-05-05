set shell := ["bash", "-cu"]

android_java_home := `if [ -x "${JAVA_HOME:-}/bin/java" ]; then printf "%s" "$JAVA_HOME"; elif [ -x /opt/android-studio/jbr/bin/java ]; then printf "%s" /opt/android-studio/jbr; elif [ -x /usr/lib/jvm/default-java/bin/java ]; then printf "%s" /usr/lib/jvm/default-java; else printf "%s" "${JAVA_HOME:-}"; fi`

default:
    just --list

# Start the local backend dependency stack and backend app.
dev:
    cd backend && just dev

# Backend checks.
backend-test:
    cd backend && just test

backend-check:
    cd backend && just check

# Admin web UI development server.
admin-dev:
    cd apps/admin && bun run dev

admin-check:
    cd apps/admin && bun run openapi:gen && bun run lint && bun run typecheck && bun run test && bun run build

# Mobile unit tests.
mobile-test:
    cd apps/mobile && JAVA_HOME="{{android_java_home}}" ./gradlew :composeApp:testDebugUnitTest

mobile-debug:
    cd apps/mobile && JAVA_HOME="{{android_java_home}}" ./gradlew :composeApp:assembleDebug

android-test: mobile-test

android-debug: mobile-debug

# Cross-project local gate.
check: backend-check admin-check mobile-test
