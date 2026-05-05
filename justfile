set shell := ["bash", "-cu"]

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
    cd tools/admin-webui && bun run dev

admin-check:
    cd tools/admin-webui && bun run openapi:gen && bun run lint && bun run typecheck && bun run test && bun run build

# Android unit tests.
android-test:
    cd apps/android && ./gradlew :composeApp:testDebugUnitTest

android-debug:
    cd apps/android && ./gradlew :composeApp:assembleDebug

# Cross-project local gate.
check: backend-check admin-check android-test
