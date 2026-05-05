import { Link, useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { Time } from "@/components/time";
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
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Users</h1>
          <p className="mt-2 text-sm text-zinc-600">Search and inspect managed accounts.</p>
        </div>
        <form
          className="flex w-full gap-2 sm:w-auto"
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
          <input
            className="min-w-0 flex-1 rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900 sm:w-72"
            id="users-search"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search by email"
            value={query}
          />
          <button className="rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white">
            Search
          </button>
          {search.q ? (
            <button
              className="rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700"
              onClick={() => {
                setQuery("");
                void navigate({
                  search: { q: undefined, cursor: undefined, limit: search.limit },
                });
              }}
              type="button"
            >
              Clear
            </button>
          ) : null}
        </form>
      </div>

      <section className="mt-6 overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-sm">
        {users.isLoading ? <UsersSkeleton /> : null}
        {users.error ? <UsersError error={users.error} /> : null}
        {users.isSuccess && items.length === 0 ? <UsersEmpty query={search.q} /> : null}
        {users.isSuccess && items.length > 0 ? <UsersTable users={items} /> : null}
      </section>

      <div className="mt-4 flex items-center justify-between">
        <p className="text-sm text-zinc-600">
          {users.isSuccess ? `${items.length} user${items.length === 1 ? "" : "s"} shown` : ""}
        </p>
        <div className="flex gap-2">
          {search.cursor ? (
            <button
              className="rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700"
              onClick={() =>
                void navigate({
                  search: { q: search.q, cursor: undefined, limit: search.limit },
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
                search: { q: search.q, cursor: nextCursor ?? undefined, limit: search.limit },
              })
            }
            type="button"
          >
            Next page
          </button>
        </div>
      </div>
    </main>
  );
}

function UsersTable({ users }: { users: ManagedUser[] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[760px] border-collapse text-left text-sm">
        <thead className="bg-zinc-50 text-xs text-zinc-500 uppercase">
          <tr>
            <th className="px-4 py-3 font-medium">Email</th>
            <th className="px-4 py-3 font-medium">Role</th>
            <th className="px-4 py-3 font-medium">Verified</th>
            <th className="px-4 py-3 font-medium">Last login</th>
            <th className="px-4 py-3 font-medium">Created</th>
            <th className="px-4 py-3 font-medium">Lock</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-zinc-200">
          {users.map((user) => (
            <tr key={user.id} className="hover:bg-zinc-50">
              <td className="px-4 py-3">
                <Link
                  className="font-medium text-zinc-950 hover:underline"
                  params={{ id: user.id }}
                  search={{ q: undefined, cursor: undefined, limit: 50 }}
                  to="/users/$id"
                >
                  {user.email}
                </Link>
                {user.display_name ? (
                  <p className="mt-1 text-xs text-zinc-500">{user.display_name}</p>
                ) : null}
              </td>
              <td className="px-4 py-3 capitalize">{formatRole(user.role)}</td>
              <td className="px-4 py-3">{user.email_verified ? "Yes" : "No"}</td>
              <td className="px-4 py-3">
                <Time value={user.last_login_at} />
              </td>
              <td className="px-4 py-3">
                <Time value={user.created_at} />
              </td>
              <td className="px-4 py-3">
                {user.locked_until ? <Time value={user.locked_until} /> : "Unlocked"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function UsersSkeleton() {
  return (
    <div className="space-y-3 p-4" aria-label="Loading users">
      {Array.from({ length: 6 }).map((_, index) => (
        <div className="h-10 animate-pulse rounded-md bg-zinc-200" key={index} />
      ))}
    </div>
  );
}

function UsersEmpty({ query }: { query: string | undefined }) {
  return (
    <div className="p-8 text-center">
      <h2 className="text-base font-semibold">No users found</h2>
      <p className="mt-2 text-sm text-zinc-600">
        {query ? `No users matching "${query}".` : "There are no managed users yet."}
      </p>
    </div>
  );
}

function UsersError({ error }: { error: Error }) {
  return (
    <div className="p-4 text-sm text-red-800" role="alert">
      {apiErrorMessage(error)}
    </div>
  );
}
