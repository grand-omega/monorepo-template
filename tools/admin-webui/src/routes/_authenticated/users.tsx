import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_authenticated/users")({
  component: UsersPage,
});

function UsersPage() {
  return (
    <main className="mx-auto max-w-6xl px-4 py-6">
      <h1 className="text-2xl font-semibold">Users</h1>
      <p className="mt-2 text-sm text-zinc-600">
        Routing is wired. The searchable user table is scheduled for roadmap step 6.
      </p>
    </main>
  );
}
