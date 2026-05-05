import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { Time } from "@/components/time";
import {
  Badge,
  Button,
  EmptyState,
  Input,
  PageHeader,
  Panel,
  SkeletonLine,
  StatusMessage,
  Table,
  TableCell,
  TableHead,
  TableHeaderCell,
} from "@/components/ui";
import { ApiError, apiErrorMessage } from "@/lib/errors";
import { Route } from "@/routes/_authenticated/auth-events";

type AuthEvent = components["schemas"]["AuthEvent"];

export function AuthEventsListPage() {
  const search = Route.useSearch();
  const navigate = useNavigate({ from: "/auth-events" });
  const [eventType, setEventType] = useState(search.event_type ?? "");
  const [userId, setUserId] = useState(search.user_id ?? "");
  const [selected, setSelected] = useState<AuthEvent | null>(null);

  const events = useQuery({
    queryKey: ["auth-events", search],
    queryFn: async () => {
      const { data, error, response } = await api.GET("/auth-events", {
        params: {
          query: {
            cursor: search.cursor,
            event_type: search.event_type,
            limit: search.limit,
            user_id: search.user_id,
          },
        },
      });
      if (error) throw new ApiError(error, response);
      return data;
    },
  });

  const items = events.data?.items ?? [];
  const nextCursor = events.data?.next_cursor;

  return (
    <main className="mx-auto max-w-6xl px-4 py-6">
      <PageHeader description="Review authentication and admin audit events." title="Auth events" />

      <form
        className="mt-6 grid gap-3 rounded-lg border border-zinc-200 bg-white p-4 shadow-sm sm:grid-cols-[1fr_1fr_auto_auto]"
        onSubmit={(event) => {
          event.preventDefault();
          void navigate({
            search: {
              cursor: undefined,
              event_type: eventType.trim() || undefined,
              limit: search.limit,
              user_id: userId.trim() || undefined,
            },
          });
        }}
      >
        <label className="block text-sm font-medium text-zinc-800" htmlFor="event-type">
          Event type
          <Input
            className="mt-1"
            data-search-input="true"
            id="event-type"
            onChange={(event) => setEventType(event.target.value)}
            placeholder="admin_login_success"
            value={eventType}
          />
        </label>
        <label className="block text-sm font-medium text-zinc-800" htmlFor="event-user-id">
          User ID
          <Input
            className="mt-1"
            id="event-user-id"
            onChange={(event) => setUserId(event.target.value)}
            placeholder="UUID"
            value={userId}
          />
        </label>
        <Button className="self-end" type="submit" variant="primary">
          Filter
        </Button>
        <Button
          className="self-end"
          onClick={() => {
            setEventType("");
            setUserId("");
            void navigate({
              search: {
                cursor: undefined,
                event_type: undefined,
                limit: search.limit,
                user_id: undefined,
              },
            });
          }}
          variant="secondary"
        >
          Clear
        </Button>
      </form>

      <Panel className="mt-6 overflow-hidden">
        {events.isLoading ? <EventsSkeleton /> : null}
        {events.error ? (
          <StatusMessage className="m-4">{apiErrorMessage(events.error)}</StatusMessage>
        ) : null}
        {events.isSuccess && items.length === 0 ? (
          <EmptyState title="No auth events found">
            <p>Adjust the filters and try again.</p>
          </EmptyState>
        ) : null}
        {events.isSuccess && items.length > 0 ? (
          <EventsTable events={items} onSelect={setSelected} />
        ) : null}
      </Panel>

      <div className="mt-4 flex items-center justify-between">
        <p className="text-sm text-zinc-600">
          {events.isSuccess ? `${items.length} event${items.length === 1 ? "" : "s"} shown` : ""}
        </p>
        <div className="flex gap-2">
          {search.cursor ? (
            <Button
              onClick={() =>
                void navigate({
                  search: {
                    cursor: undefined,
                    event_type: search.event_type,
                    limit: search.limit,
                    user_id: search.user_id,
                  },
                })
              }
              variant="secondary"
            >
              First page
            </Button>
          ) : null}
          <Button
            disabled={!nextCursor}
            onClick={() =>
              void navigate({
                search: {
                  cursor: nextCursor ?? undefined,
                  event_type: search.event_type,
                  limit: search.limit,
                  user_id: search.user_id,
                },
              })
            }
            variant="primary"
          >
            Next page
          </Button>
        </div>
      </div>

      <EventDetailDrawer event={selected} onClose={() => setSelected(null)} />
    </main>
  );
}

function EventsTable({
  events,
  onSelect,
}: {
  events: AuthEvent[];
  onSelect: (event: AuthEvent) => void;
}) {
  return (
    <div className="overflow-x-auto">
      <Table className="min-w-[820px]">
        <TableHead>
          <tr>
            <TableHeaderCell>Created</TableHeaderCell>
            <TableHeaderCell>Event type</TableHeaderCell>
            <TableHeaderCell>User</TableHeaderCell>
            <TableHeaderCell>IP</TableHeaderCell>
            <TableHeaderCell>Details</TableHeaderCell>
          </tr>
        </TableHead>
        <tbody className="divide-y divide-zinc-200">
          {events.map((event) => (
            <tr key={event.id} className="transition-colors hover:bg-zinc-50">
              <TableCell>
                <Time value={event.created_at} />
              </TableCell>
              <TableCell>
                <Badge tone={event.event_type.includes("fail") ? "danger" : "info"}>
                  {event.event_type}
                </Badge>
              </TableCell>
              <TableCell>
                <span>{event.user_email ?? event.user_id ?? "Unknown"}</span>
              </TableCell>
              <TableCell>{event.ip ?? "Unknown"}</TableCell>
              <TableCell>
                <Button onClick={() => onSelect(event)} size="sm" variant="secondary">
                  View
                </Button>
              </TableCell>
            </tr>
          ))}
        </tbody>
      </Table>
    </div>
  );
}

function EventDetailDrawer({ event, onClose }: { event: AuthEvent | null; onClose: () => void }) {
  if (!event) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/30" role="presentation">
      <aside
        aria-label="Auth event detail"
        className="ml-auto h-full w-full max-w-xl overflow-y-auto border-l border-zinc-200 bg-white p-6 shadow-xl"
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold">{event.event_type}</h2>
            <p className="mt-1 text-sm text-zinc-600">{event.id}</p>
          </div>
          <Button onClick={onClose} variant="secondary">
            Close
          </Button>
        </div>
        <dl className="mt-6 grid gap-4">
          <Detail label="Created">
            <Time value={event.created_at} />
          </Detail>
          <Detail label="User" value={event.user_email ?? event.user_id ?? "Unknown"} />
          <Detail label="IP" value={event.ip ?? "Unknown"} />
          <Detail label="User agent" value={event.user_agent ?? "Unknown"} />
          <Detail label="Detail JSON">
            <pre className="overflow-x-auto rounded-md border border-zinc-800 bg-zinc-950 p-3 text-xs text-white">
              {JSON.stringify(event.detail ?? null, null, 2)}
            </pre>
          </Detail>
        </dl>
      </aside>
    </div>
  );
}

function Detail({
  children,
  label,
  value,
}: {
  children?: ReactNode;
  label: string;
  value?: string;
}) {
  return (
    <div>
      <dt className="text-xs font-medium text-zinc-500 uppercase">{label}</dt>
      <dd className="mt-1 text-sm break-words text-zinc-950">{children ?? value}</dd>
    </div>
  );
}

function EventsSkeleton() {
  return (
    <div className="space-y-3 p-4" aria-label="Loading auth events">
      {Array.from({ length: 8 }).map((_, index) => (
        <SkeletonLine className="h-10" key={index} />
      ))}
    </div>
  );
}
