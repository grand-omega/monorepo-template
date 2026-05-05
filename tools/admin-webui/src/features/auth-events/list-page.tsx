import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { Time } from "@/components/time";
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
      <div>
        <h1 className="text-2xl font-semibold">Auth events</h1>
        <p className="mt-2 text-sm text-zinc-600">Review authentication and admin audit events.</p>
      </div>

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
        <label className="block text-sm font-medium text-zinc-800">
          Event type
          <input
            className="mt-1 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900"
            data-search-input="true"
            onChange={(event) => setEventType(event.target.value)}
            placeholder="admin_login_success"
            value={eventType}
          />
        </label>
        <label className="block text-sm font-medium text-zinc-800">
          User ID
          <input
            className="mt-1 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900"
            onChange={(event) => setUserId(event.target.value)}
            placeholder="UUID"
            value={userId}
          />
        </label>
        <button className="self-end rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white">
          Filter
        </button>
        <button
          className="self-end rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700"
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
          type="button"
        >
          Clear
        </button>
      </form>

      <section className="mt-6 overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-sm">
        {events.isLoading ? <EventsSkeleton /> : null}
        {events.error ? (
          <div className="p-4 text-sm text-red-800" role="alert">
            {apiErrorMessage(events.error)}
          </div>
        ) : null}
        {events.isSuccess && items.length === 0 ? (
          <div className="p-8 text-center">
            <h2 className="text-base font-semibold">No auth events found</h2>
            <p className="mt-2 text-sm text-zinc-600">Adjust the filters and try again.</p>
          </div>
        ) : null}
        {events.isSuccess && items.length > 0 ? (
          <EventsTable events={items} onSelect={setSelected} />
        ) : null}
      </section>

      <div className="mt-4 flex items-center justify-between">
        <p className="text-sm text-zinc-600">
          {events.isSuccess ? `${items.length} event${items.length === 1 ? "" : "s"} shown` : ""}
        </p>
        <div className="flex gap-2">
          {search.cursor ? (
            <button
              className="rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700"
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
              type="button"
            >
              First page
            </button>
          ) : null}
          <button
            className="rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-50"
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
            type="button"
          >
            Next page
          </button>
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
      <table className="w-full min-w-[820px] border-collapse text-left text-sm">
        <thead className="bg-zinc-50 text-xs text-zinc-500 uppercase">
          <tr>
            <th className="px-4 py-3 font-medium">Created</th>
            <th className="px-4 py-3 font-medium">Event type</th>
            <th className="px-4 py-3 font-medium">User</th>
            <th className="px-4 py-3 font-medium">IP</th>
            <th className="px-4 py-3 font-medium">Details</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-zinc-200">
          {events.map((event) => (
            <tr key={event.id} className="hover:bg-zinc-50">
              <td className="px-4 py-3">
                <Time value={event.created_at} />
              </td>
              <td className="px-4 py-3 font-medium">{event.event_type}</td>
              <td className="px-4 py-3">
                <span>{event.user_email ?? event.user_id ?? "Unknown"}</span>
              </td>
              <td className="px-4 py-3">{event.ip ?? "Unknown"}</td>
              <td className="px-4 py-3">
                <button
                  className="rounded-md border border-zinc-300 px-2 py-1 text-xs text-zinc-700"
                  onClick={() => onSelect(event)}
                  type="button"
                >
                  View
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EventDetailDrawer({ event, onClose }: { event: AuthEvent | null; onClose: () => void }) {
  if (!event) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/30" role="presentation">
      <aside
        aria-label="Auth event detail"
        className="ml-auto h-full w-full max-w-xl overflow-y-auto bg-white p-6 shadow-xl"
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold">{event.event_type}</h2>
            <p className="mt-1 text-sm text-zinc-600">{event.id}</p>
          </div>
          <button
            className="rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700"
            onClick={onClose}
          >
            Close
          </button>
        </div>
        <dl className="mt-6 grid gap-4">
          <Detail label="Created">
            <Time value={event.created_at} />
          </Detail>
          <Detail label="User" value={event.user_email ?? event.user_id ?? "Unknown"} />
          <Detail label="IP" value={event.ip ?? "Unknown"} />
          <Detail label="User agent" value={event.user_agent ?? "Unknown"} />
          <Detail label="Detail JSON">
            <pre className="overflow-x-auto rounded-md bg-zinc-950 p-3 text-xs text-white">
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
        <div className="h-10 animate-pulse rounded-md bg-zinc-200" key={index} />
      ))}
    </div>
  );
}
