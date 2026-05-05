import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_authenticated/auth-events")({
  component: AuthEventsPage,
});

function AuthEventsPage() {
  return (
    <main className="mx-auto max-w-6xl px-4 py-6">
      <h1 className="text-2xl font-semibold">Auth events</h1>
      <p className="mt-2 text-sm text-zinc-600">
        Routing is wired. The filterable audit table is scheduled for roadmap step 8.
      </p>
    </main>
  );
}
