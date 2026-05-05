import { Link, useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
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
import { formatRole } from "@/lib/format";
import { Route } from "@/routes/_authenticated/users";

type ManagedUser = components["schemas"]["ManagedUser"];

export function UsersListPage() {
  const search = Route.useSearch();
  const navigate = useNavigate({ from: "/users" });
  const [query, setQuery] = useState(search.q ?? "");

  const users = useQuery({
    queryKey: ["users", search],
    queryFn: async () => {
      const { data, error, response } = await api.GET("/users", {
        params: {
          query: {
            q: search.q,
            cursor: search.cursor,
            limit: search.limit,
          },
        },
      });
      if (error) throw new ApiError(error, response);
      return data;
    },
  });

  const items = users.data?.items ?? [];
  const nextCursor = users.data?.next_cursor;

  return (
    <main className="mx-auto max-w-6xl px-4 py-6">
      <PageHeader description="Search and inspect managed accounts." title="Users">
        <form
          className="mt-4 flex w-full flex-wrap gap-2 sm:max-w-xl"
          onSubmit={(event) => {
            event.preventDefault();
            void navigate({
              search: {
                q: query.trim() || undefined,
                cursor: undefined,
                limit: search.limit,
              },
            });
          }}
        >
          <label className="sr-only" htmlFor="users-search">
            Search users
          </label>
          <Input
            className="min-w-0 flex-1 sm:w-72"
            id="users-search"
            data-search-input="true"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search by email"
            value={query}
          />
          <Button type="submit" variant="primary">
            Search
          </Button>
          {search.q ? (
            <Button
              onClick={() => {
                setQuery("");
                void navigate({
                  search: { q: undefined, cursor: undefined, limit: search.limit },
                });
              }}
              variant="secondary"
            >
              Clear
            </Button>
          ) : null}
        </form>
      </PageHeader>

      <Panel className="mt-6 overflow-hidden">
        {users.isLoading ? <UsersSkeleton /> : null}
        {users.error ? <UsersError error={users.error} /> : null}
        {users.isSuccess && items.length === 0 ? <UsersEmpty query={search.q} /> : null}
        {users.isSuccess && items.length > 0 ? <UsersTable users={items} /> : null}
      </Panel>

      <div className="mt-4 flex items-center justify-between">
        <p className="text-sm text-zinc-600">
          {users.isSuccess ? `${items.length} user${items.length === 1 ? "" : "s"} shown` : ""}
        </p>
        <div className="flex gap-2">
          {search.cursor ? (
            <Button
              onClick={() =>
                void navigate({
                  search: { q: search.q, cursor: undefined, limit: search.limit },
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
                search: { q: search.q, cursor: nextCursor ?? undefined, limit: search.limit },
              })
            }
            variant="primary"
          >
            Next page
          </Button>
        </div>
      </div>
    </main>
  );
}

function UsersTable({ users }: { users: ManagedUser[] }) {
  return (
    <div className="overflow-x-auto">
      <Table className="min-w-[760px]">
        <TableHead>
          <tr>
            <TableHeaderCell>Email</TableHeaderCell>
            <TableHeaderCell>Role</TableHeaderCell>
            <TableHeaderCell>Verified</TableHeaderCell>
            <TableHeaderCell>Last login</TableHeaderCell>
            <TableHeaderCell>Created</TableHeaderCell>
            <TableHeaderCell>Lock</TableHeaderCell>
          </tr>
        </TableHead>
        <tbody className="divide-y divide-zinc-200">
          {users.map((user) => (
            <tr key={user.id} className="transition-colors hover:bg-zinc-50">
              <TableCell>
                <Link
                  className="font-medium text-zinc-950 underline-offset-2 hover:underline focus:ring-4 focus:ring-zinc-950/10 focus:outline-none"
                  params={{ id: user.id }}
                  search={{ q: undefined, cursor: undefined, limit: 50 }}
                  to="/users/$id"
                >
                  {user.email}
                </Link>
                {user.display_name ? (
                  <p className="mt-1 text-xs text-zinc-500">{user.display_name}</p>
                ) : null}
              </TableCell>
              <TableCell>
                <Badge>{formatRole(user.role)}</Badge>
              </TableCell>
              <TableCell>
                <Badge tone={user.email_verified ? "success" : "warning"}>
                  {user.email_verified ? "Yes" : "No"}
                </Badge>
              </TableCell>
              <TableCell>
                <Time value={user.last_login_at} />
              </TableCell>
              <TableCell>
                <Time value={user.created_at} />
              </TableCell>
              <TableCell>
                {user.locked_until ? (
                  <Badge tone="danger">
                    <Time value={user.locked_until} />
                  </Badge>
                ) : (
                  <Badge tone="success">Unlocked</Badge>
                )}
              </TableCell>
            </tr>
          ))}
        </tbody>
      </Table>
    </div>
  );
}

function UsersSkeleton() {
  return (
    <div className="space-y-3 p-4" aria-label="Loading users">
      {Array.from({ length: 6 }).map((_, index) => (
        <SkeletonLine className="h-10" key={index} />
      ))}
    </div>
  );
}

function UsersEmpty({ query }: { query: string | undefined }) {
  return (
    <EmptyState title="No users found">
      <p>{query ? `No users matching "${query}".` : "There are no managed users yet."}</p>
    </EmptyState>
  );
}

function UsersError({ error }: { error: Error }) {
  return <StatusMessage className="m-4">{apiErrorMessage(error)}</StatusMessage>;
}
