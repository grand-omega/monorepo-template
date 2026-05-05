import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import jsxA11y from "eslint-plugin-jsx-a11y";
import security from "eslint-plugin-security";

export default tseslint.config(
  {
    ignores: [
      "dist",
      "node_modules",
      "playwright-report",
      "test-results",
      "coverage",
      "src/api/schema.ts",
      "src/routeTree.gen.ts",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strict,
  ...tseslint.configs.stylistic,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: { ...globals.browser, ...globals.node },
      parserOptions: {
        ecmaFeatures: { jsx: true },
      },
    },
    plugins: {
      react,
      "react-hooks": reactHooks,
      "jsx-a11y": jsxA11y,
      security,
    },
    settings: {
      react: { version: "detect" },
    },
    rules: {
      ...react.configs.recommended.rules,
      ...react.configs["jsx-runtime"].rules,
      ...reactHooks.configs.recommended.rules,
      ...jsxA11y.configs.recommended.rules,
      ...security.configs.recommended.rules,

      "@typescript-eslint/consistent-type-imports": "error",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],

      "no-console": ["error", { allow: ["warn", "error"] }],
      "no-restricted-globals": [
        "error",
        { name: "fetch", message: "Use the api client from src/api/client.ts." },
      ],
    },
  },
  {
    files: ["tests/**/*.{ts,tsx}", "**/*.{test,spec}.{ts,tsx}"],
    rules: {
      "no-console": "off",
      "no-restricted-globals": "off",
    },
  },
  {
    files: ["vite.config.ts", "playwright.config.ts", "eslint.config.js"],
    languageOptions: { globals: globals.node },
  },
  {
    files: ["scripts/**/*.{mjs,js,ts}"],
    languageOptions: { globals: { ...globals.node, fetch: "readonly" } },
    rules: {
      "no-console": "off",
      "no-restricted-globals": "off",
    },
  },
  {
    // src/auth/webauthn.ts uses raw fetch because the WebAuthn endpoints aren't
    // in the committed OpenAPI snapshot yet. After running `bun run openapi`
    // against a backend that includes /admin/api/webauthn/*, migrate this file
    // to the openapi-fetch `api` client and remove this override.
    files: ["src/auth/webauthn.ts"],
    rules: { "no-restricted-globals": "off" },
  }
);
