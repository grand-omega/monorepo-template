/// <reference types="vitest" />
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
import path from "node:path";

function isNodePackage(id: string, packageName: string) {
  const normalized = id.replaceAll("\\", "/");
  const marker = "/node_modules/";
  const markerIndex = normalized.lastIndexOf(marker);
  if (markerIndex === -1) return false;

  const segments = normalized.slice(markerIndex + marker.length).split("/");
  const installedPackage =
    segments[0]?.startsWith("@") && segments[1] ? `${segments[0]}/${segments[1]}` : segments[0];

  return installedPackage === packageName;
}

export default defineConfig({
  // Router plugin must run before react() so generated routes are visible.
  plugins: [
    TanStackRouterVite({ target: "react", autoCodeSplitting: true }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
  base: "/admin/",
  server: {
    port: 5173,
    proxy: {
      "/admin/api": {
        target: process.env.VITE_API_URL ?? "http://localhost:8080",
        changeOrigin: false,
      },
    },
  },
  build: {
    outDir: "dist",
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (isNodePackage(id, "react") || isNodePackage(id, "react-dom")) {
            return "react";
          }
          if (isNodePackage(id, "@tanstack/react-router")) {
            return "router";
          }
          if (isNodePackage(id, "@tanstack/react-query")) {
            return "query";
          }
        },
      },
    },
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./tests/unit/setup.ts"],
    include: ["tests/unit/**/*.{test,spec}.{ts,tsx}", "src/**/*.{test,spec}.{ts,tsx}"],
    exclude: ["node_modules", "dist", "tests/e2e/**"],
  },
});
