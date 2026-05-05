# Development

Use the root `justfile` for common tasks. Subproject-specific workflows remain documented in each app README.

Daily business-logic work should target local or staging environments, not production.

Common commands:

```sh
just dev
just backend-check
just admin-check
just mobile-test
just check
```
