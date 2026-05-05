import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_authenticated/users/$id")({
  component: UserDetailPage,
});

function UserDetailPage() {
  const { id } = Route.useParams();

  return (
    <main className="mx-auto max-w-6xl px-4 py-6">
      <h1 className="text-2xl font-semibold">User detail</h1>
      <p className="mt-2 text-sm text-zinc-600">Selected user: {id}</p>
    </main>
  );
}
