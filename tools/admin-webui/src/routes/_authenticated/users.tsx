import { Outlet, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_authenticated/users")({
  validateSearch: (search: Record<string, unknown>) => ({
    q: typeof search.q === "string" && search.q.length > 0 ? search.q : undefined,
    cursor:
      typeof search.cursor === "string" && search.cursor.length > 0 ? search.cursor : undefined,
    limit: parseLimit(search.limit),
  }),
  component: UsersLayout,
});

function UsersLayout() {
  return <Outlet />;
}

function parseLimit(value: unknown): number {
  const parsed = typeof value === "string" ? Number(value) : value;
  if (typeof parsed !== "number" || !Number.isFinite(parsed)) return 50;
  return Math.min(Math.max(Math.trunc(parsed), 1), 200);
}
