import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { queryKeys } from "@/api/queries";
import { Time } from "@/components/time";
import { ApiError, apiErrorMessage } from "@/lib/errors";

type UserSession = components["schemas"]["UserSession"];

interface SessionsListProps {
  userId: string;
}

export function SessionsList({ userId }: SessionsListProps) {
  const sessions = useQuery({
    queryKey: queryKeys.userSessions(userId),
    queryFn: async () => {
      const { data, error, response } = await api.GET("/users/{id}/sessions", {
        params: { path: { id: userId } },
      });
      if (error) throw new ApiError(error, response);
      return data;
    },
  });

  return (
    <section className="mt-6 rounded-lg border border-zinc-200 bg-white p-6 shadow-sm">
      <div>
        <h2 className="text-lg font-semibold">Sessions</h2>
        <p className="mt-1 text-sm text-zinc-600">Refresh-token families for this user.</p>
      </div>

      {sessions.isLoading ? <SessionsSkeleton /> : null}
      {sessions.error ? (
        <p className="mt-4 rounded-md bg-red-50 px-3 py-2 text-sm text-red-800" role="alert">
          {apiErrorMessage(sessions.error)}
        </p>
      ) : null}
      {sessions.isSuccess && sessions.data.items.length === 0 ? (
        <p className="mt-4 rounded-md bg-zinc-50 px-3 py-2 text-sm text-zinc-600">
          No refresh sessions found.
        </p>
      ) : null}
      {sessions.isSuccess && sessions.data.items.length > 0 ? (
        <SessionsTable sessions={sessions.data.items} />
      ) : null}
    </section>
  );
}

function SessionsTable({ sessions }: { sessions: UserSession[] }) {
  return (
    <div className="mt-4 overflow-x-auto">
      <table className="w-full min-w-[760px] border-collapse text-left text-sm">
        <thead className="bg-zinc-50 text-xs text-zinc-500 uppercase">
          <tr>
            <th className="px-4 py-3 font-medium">Family ID</th>
            <th className="px-4 py-3 font-medium">Created</th>
            <th className="px-4 py-3 font-medium">Last used</th>
            <th className="px-4 py-3 font-medium">Revoked</th>
            <th className="px-4 py-3 font-medium">IP</th>
            <th className="px-4 py-3 font-medium">User agent</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-zinc-200">
          {sessions.map((session) => (
            <tr key={session.id}>
              <td className="max-w-48 truncate px-4 py-3 font-mono text-xs" title={session.id}>
                {session.id}
              </td>
              <td className="px-4 py-3">
                <Time value={session.created_at} />
              </td>
              <td className="px-4 py-3">
                <Time value={session.last_used_at} />
              </td>
              <td className="px-4 py-3">
                {session.revoked_at ? <Time value={session.revoked_at} /> : "Active"}
              </td>
              <td className="px-4 py-3">{session.ip ?? "Unknown"}</td>
              <td className="max-w-64 truncate px-4 py-3" title={session.user_agent ?? undefined}>
                {session.user_agent ?? "Unknown"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function SessionsSkeleton() {
  return (
    <div className="mt-4 space-y-3" aria-label="Loading user sessions">
      {Array.from({ length: 3 }).map((_, index) => (
        <div className="h-10 animate-pulse rounded-md bg-zinc-200" key={index} />
      ))}
    </div>
  );
}
