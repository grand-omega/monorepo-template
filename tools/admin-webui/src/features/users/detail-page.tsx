import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { queryKeys } from "@/api/queries";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Time } from "@/components/time";
import { ApiError, apiErrorMessage } from "@/lib/errors";
import { formatOptionalDateTime, formatRole } from "@/lib/format";
import { Route } from "@/routes/_authenticated/users.$id";

type ManagedUser = components["schemas"]["ManagedUser"];

export function UserDetailPage() {
  const { id } = Route.useParams();
  const user = useQuery({
    queryKey: queryKeys.user(id),
    queryFn: async () => {
      const { data, error, response } = await api.GET("/users/{id}", {
        params: { path: { id } },
      });
      if (error) throw new ApiError(error, response);
      return data;
    },
  });

  if (user.isLoading) return <UserDetailSkeleton />;

  if (user.error) {
    return (
      <main className="mx-auto max-w-6xl px-4 py-6">
        <Link
          className="text-sm text-zinc-600 hover:text-zinc-950"
          search={{ q: undefined, cursor: undefined, limit: 50 }}
          to="/users"
        >
          Back to users
        </Link>
        <div
          className="mt-4 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-800"
          role="alert"
        >
          {apiErrorMessage(user.error)}
        </div>
      </main>
    );
  }

  if (!user.data) return null;

  return <UserDetailContent user={user.data} />;
}

function UserDetailContent({ user }: { user: ManagedUser }) {
  return (
    <main className="mx-auto max-w-6xl px-4 py-6">
      <Link
        className="text-sm text-zinc-600 hover:text-zinc-950"
        search={{ q: undefined, cursor: undefined, limit: 50 }}
        to="/users"
      >
        Back to users
      </Link>
      <div className="mt-4 grid gap-6 lg:grid-cols-[1fr_320px]">
        <section className="rounded-lg border border-zinc-200 bg-white p-6 shadow-sm">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h1 className="text-2xl font-semibold">{user.email}</h1>
              {user.display_name ? (
                <p className="mt-1 text-sm text-zinc-600">{user.display_name}</p>
              ) : null}
            </div>
            <span className="w-fit rounded-md bg-zinc-100 px-2 py-1 text-xs font-medium text-zinc-700 capitalize">
              {formatRole(user.role)}
            </span>
          </div>

          <dl className="mt-6 grid gap-4 sm:grid-cols-2">
            <Field label="User ID" value={user.id} />
            <Field label="Email verified" value={user.email_verified ? "Yes" : "No"} />
            <Field label="Failed logins" value={String(user.failed_login_count)} />
            <Field label="Locked until" value={formatOptionalDateTime(user.locked_until)} />
            <Field label="Created">
              <Time value={user.created_at} />
            </Field>
            <Field label="Last login">
              <Time value={user.last_login_at} />
            </Field>
          </dl>
        </section>

        <UserActions user={user} />
      </div>
    </main>
  );
}

function Field({
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

function UserActions({ user }: { user: ManagedUser }) {
  const queryClient = useQueryClient();
  const [dialog, setDialog] = useState<"lock" | "unlock" | "verify" | "revoke" | null>(null);
  const [reason, setReason] = useState("");

  function userKey() {
    return queryKeys.user(user.id);
  }

  const lock = useMutation({
    mutationFn: async () => {
      const { error, response } = await api.POST("/users/{id}/lock", {
        params: { path: { id: user.id } },
        body: { reason: reason.trim() || undefined },
      });
      if (error) throw new ApiError(error, response);
    },
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: userKey() });
      const previous = queryClient.getQueryData<ManagedUser>(userKey());
      queryClient.setQueryData<ManagedUser>(userKey(), (current) =>
        current
          ? {
              ...current,
              locked_until: new Date(Date.now() + 10 * 365 * 24 * 60 * 60 * 1000).toISOString(),
            }
          : current
      );
      return { previous };
    },
    onError: (_error, _variables, context) => {
      if (context?.previous) queryClient.setQueryData(userKey(), context.previous);
    },
    onSettled: async () => {
      setDialog(null);
      setReason("");
      await queryClient.invalidateQueries({ queryKey: userKey() });
    },
  });

  const unlock = useOptimisticUserAction(
    user,
    async () => {
      const { error, response } = await api.POST("/users/{id}/unlock", {
        params: { path: { id: user.id } },
      });
      if (error) throw new ApiError(error, response);
    },
    (current) => ({ ...current, failed_login_count: 0, locked_until: null }),
    () => setDialog(null)
  );

  const verify = useOptimisticUserAction(
    user,
    async () => {
      const { error, response } = await api.POST("/users/{id}/verify-email", {
        params: { path: { id: user.id } },
      });
      if (error) throw new ApiError(error, response);
    },
    (current) => ({ ...current, email_verified: true }),
    () => setDialog(null)
  );

  const revoke = useMutation({
    mutationFn: async () => {
      const { error, response } = await api.DELETE("/users/{id}/sessions", {
        params: { path: { id: user.id } },
      });
      if (error) throw new ApiError(error, response);
    },
    onSettled: () => setDialog(null),
  });

  const pending = lock.isPending || unlock.isPending || verify.isPending || revoke.isPending;
  const error = lock.error ?? unlock.error ?? verify.error ?? revoke.error;

  return (
    <aside className="rounded-lg border border-zinc-200 bg-white p-5 shadow-sm">
      <h2 className="text-base font-semibold">Actions</h2>
      <p className="mt-1 text-sm text-zinc-600">Confirm each action for {user.email}.</p>
      {error ? (
        <p className="mt-3 rounded-md bg-red-50 px-3 py-2 text-sm text-red-800" role="alert">
          {apiErrorMessage(error)}
        </p>
      ) : null}
      <div className="mt-4 grid gap-2">
        {user.locked_until ? (
          <button
            className="rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white"
            onClick={() => setDialog("unlock")}
            type="button"
          >
            Unlock user
          </button>
        ) : (
          <button
            className="rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white"
            onClick={() => setDialog("lock")}
            type="button"
          >
            Lock user
          </button>
        )}
        <button
          className="rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={user.email_verified}
          onClick={() => setDialog("verify")}
          type="button"
        >
          Verify email
        </button>
        <button
          className="rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700"
          onClick={() => setDialog("revoke")}
          type="button"
        >
          Revoke sessions
        </button>
      </div>

      <ConfirmDialog
        confirmLabel={pending ? "Locking..." : "Lock user"}
        description={`Lock ${user.email}. The backend default expiration will be used.`}
        disabled={pending}
        onCancel={() => setDialog(null)}
        onConfirm={() => lock.mutate()}
        open={dialog === "lock"}
        title="Lock user"
      >
        <label className="block text-sm font-medium text-zinc-800">
          Reason
          <textarea
            className="mt-1 min-h-24 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900"
            onChange={(event) => setReason(event.target.value)}
            value={reason}
          />
        </label>
      </ConfirmDialog>
      <ConfirmDialog
        confirmLabel={pending ? "Unlocking..." : "Unlock user"}
        description={`Unlock ${user.email}.`}
        disabled={pending}
        onCancel={() => setDialog(null)}
        onConfirm={() => unlock.mutate()}
        open={dialog === "unlock"}
        title="Unlock user"
      />
      <ConfirmDialog
        confirmLabel={pending ? "Verifying..." : "Verify email"}
        description={`Mark ${user.email} as email verified.`}
        disabled={pending}
        onCancel={() => setDialog(null)}
        onConfirm={() => verify.mutate()}
        open={dialog === "verify"}
        title="Verify email"
      />
      <ConfirmDialog
        confirmLabel={pending ? "Revoking..." : "Revoke sessions"}
        description={`Revoke all refresh-token sessions for ${user.email}.`}
        disabled={pending}
        onCancel={() => setDialog(null)}
        onConfirm={() => revoke.mutate()}
        open={dialog === "revoke"}
        title="Revoke sessions"
      />
    </aside>
  );
}

function useOptimisticUserAction(
  user: ManagedUser,
  mutationFn: () => Promise<void>,
  optimisticUpdate: (current: ManagedUser) => ManagedUser,
  onSettled?: () => void
) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn,
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: queryKeys.user(user.id) });
      const previous = queryClient.getQueryData<ManagedUser>(queryKeys.user(user.id));
      queryClient.setQueryData<ManagedUser>(queryKeys.user(user.id), (current) =>
        current ? optimisticUpdate(current) : current
      );
      return { previous };
    },
    onError: (_error, _variables, context) => {
      if (context?.previous) queryClient.setQueryData(queryKeys.user(user.id), context.previous);
    },
    onSettled: async () => {
      onSettled?.();
      await queryClient.invalidateQueries({ queryKey: queryKeys.user(user.id) });
    },
  });
}

function UserDetailSkeleton() {
  return (
    <main className="mx-auto max-w-6xl px-4 py-6">
      <div className="h-5 w-24 animate-pulse rounded bg-zinc-200" />
      <div className="mt-4 grid gap-6 lg:grid-cols-[1fr_320px]">
        <div className="h-72 animate-pulse rounded-lg bg-zinc-200" />
        <div className="h-64 animate-pulse rounded-lg bg-zinc-200" />
      </div>
    </main>
  );
}
