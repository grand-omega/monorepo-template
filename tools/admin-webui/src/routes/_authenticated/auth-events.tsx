import { createFileRoute } from "@tanstack/react-router";
import { AuthEventsListPage } from "@/features/auth-events/list-page";

export const Route = createFileRoute("/_authenticated/auth-events")({
  validateSearch: (search: Record<string, unknown>) => ({
    cursor:
      typeof search.cursor === "string" && search.cursor.length > 0 ? search.cursor : undefined,
    event_type:
      typeof search.event_type === "string" && search.event_type.length > 0
        ? search.event_type
        : undefined,
    limit: parseLimit(search.limit),
    user_id:
      typeof search.user_id === "string" && search.user_id.length > 0 ? search.user_id : undefined,
  }),
  component: AuthEventsListPage,
});

function parseLimit(value: unknown): number {
  const parsed = typeof value === "string" ? Number(value) : value;
  if (typeof parsed !== "number" || !Number.isFinite(parsed)) return 100;
  return Math.min(Math.max(Math.trunc(parsed), 1), 500);
}
