import { Link, Outlet, createFileRoute, redirect } from "@tanstack/react-router";
import { api } from "@/api/client";

export const Route = createFileRoute("/_authenticated")({
  beforeLoad: async ({ context, location }) => {
    const session = await context.queryClient.fetchQuery({
      queryKey: ["me"],
      queryFn: async () => {
        const { data, error, response } = await api.GET("/me");
        if (error) {
          if (response.status === 401) {
            throw redirect({
              to: "/login",
              search: { redirect: location.href },
            });
          }
          throw new Error(error.message);
        }
        return data;
      },
    });

    return { session };
  },
  component: AuthenticatedLayout,
  pendingComponent: AuthenticatedSkeleton,
});

function AuthenticatedLayout() {
  const { session } = Route.useRouteContext();

  return (
    <div className="min-h-dvh bg-stone-50 text-zinc-950">
      <header className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-3">
          <div>
            <p className="text-sm font-semibold">lab-rust-server admin</p>
            <p className="text-xs text-zinc-500">{session.email}</p>
          </div>
          <nav aria-label="Admin navigation" className="flex items-center gap-2 text-sm">
            <Link
              activeProps={{ className: "bg-zinc-900 text-white" }}
              className="rounded-md px-3 py-2 text-zinc-700 hover:bg-zinc-100"
              to="/users"
            >
              Users
            </Link>
            <Link
              activeProps={{ className: "bg-zinc-900 text-white" }}
              className="rounded-md px-3 py-2 text-zinc-700 hover:bg-zinc-100"
              to="/auth-events"
            >
              Auth events
            </Link>
          </nav>
        </div>
      </header>
      <Outlet />
    </div>
  );
}

function AuthenticatedSkeleton() {
  return (
    <main className="min-h-dvh bg-stone-50 px-4 py-6">
      <div className="mx-auto max-w-6xl space-y-4">
        <div className="h-10 w-64 animate-pulse rounded-md bg-zinc-200" />
        <div className="h-40 animate-pulse rounded-lg bg-zinc-200" />
      </div>
    </main>
  );
}
