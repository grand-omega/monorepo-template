import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { queryKeys } from "@/api/queries";
import { Time } from "@/components/time";
import {
  Badge,
  Panel,
  SkeletonLine,
  StatusMessage,
  Table,
  TableCell,
  TableHead,
  TableHeaderCell,
} from "@/components/ui";
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
    <Panel className="mt-6 p-6">
      <div>
        <h2 className="text-lg font-semibold">Sessions</h2>
        <p className="mt-1 text-sm text-zinc-600">Refresh-token families for this user.</p>
      </div>

      {sessions.isLoading ? <SessionsSkeleton /> : null}
      {sessions.error ? (
        <StatusMessage className="mt-4">{apiErrorMessage(sessions.error)}</StatusMessage>
      ) : null}
      {sessions.isSuccess && sessions.data.items.length === 0 ? (
        <StatusMessage className="mt-4" tone="muted">
          No refresh sessions found.
        </StatusMessage>
      ) : null}
      {sessions.isSuccess && sessions.data.items.length > 0 ? (
        <SessionsTable sessions={sessions.data.items} />
      ) : null}
    </Panel>
  );
}

function SessionsTable({ sessions }: { sessions: UserSession[] }) {
  return (
    <div className="mt-4 overflow-x-auto">
      <Table className="min-w-[760px] overflow-hidden rounded-md border border-zinc-200">
        <TableHead>
          <tr>
            <TableHeaderCell>Family ID</TableHeaderCell>
            <TableHeaderCell>Created</TableHeaderCell>
            <TableHeaderCell>Last used</TableHeaderCell>
            <TableHeaderCell>Revoked</TableHeaderCell>
            <TableHeaderCell>IP</TableHeaderCell>
            <TableHeaderCell>User agent</TableHeaderCell>
          </tr>
        </TableHead>
        <tbody className="divide-y divide-zinc-200">
          {sessions.map((session) => (
            <tr key={session.id} className="transition-colors hover:bg-zinc-50">
              <TableCell className="max-w-48 truncate font-mono text-xs" title={session.id}>
                {session.id}
              </TableCell>
              <TableCell>
                <Time value={session.created_at} />
              </TableCell>
              <TableCell>
                <Time value={session.last_used_at} />
              </TableCell>
              <TableCell>
                {session.revoked_at ? (
                  <Badge tone="warning">
                    <Time value={session.revoked_at} />
                  </Badge>
                ) : (
                  <Badge tone="success">Active</Badge>
                )}
              </TableCell>
              <TableCell>{session.ip ?? "Unknown"}</TableCell>
              <TableCell className="max-w-64 truncate" title={session.user_agent ?? undefined}>
                {session.user_agent ?? "Unknown"}
              </TableCell>
            </tr>
          ))}
        </tbody>
      </Table>
    </div>
  );
}

function SessionsSkeleton() {
  return (
    <div className="mt-4 space-y-3" aria-label="Loading user sessions">
      {Array.from({ length: 3 }).map((_, index) => (
        <SkeletonLine className="h-10" key={index} />
      ))}
    </div>
  );
}
