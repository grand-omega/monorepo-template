import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ArrowLeft, ShieldAlert } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { queryKeys } from "@/api/queries";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Time } from "@/components/time";
import { useToast } from "@/components/toast";
import {
  Badge,
  Button,
  Panel,
  SkeletonLine,
  StatusMessage,
  Textarea,
  buttonClassName,
} from "@/components/ui";
import { SessionsList } from "@/features/users/sessions-list";
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
          className={buttonClassName({ variant: "ghost" })}
          search={{ q: undefined, cursor: undefined, limit: 50 }}
          to="/users"
        >
          <ArrowLeft className="size-4" aria-hidden="true" />
          Back to users
        </Link>
        <StatusMessage className="mt-4">{apiErrorMessage(user.error)}</StatusMessage>
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
        className={buttonClassName({ variant: "ghost" })}
        search={{ q: undefined, cursor: undefined, limit: 50 }}
        to="/users"
      >
        <ArrowLeft className="size-4" aria-hidden="true" />
        Back to users
      </Link>
      <div className="mt-4 grid gap-6 lg:grid-cols-[1fr_320px]">
        <div>
          <Panel className="p-6">
            <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <h1 className="text-2xl font-semibold">{user.email}</h1>
                {user.display_name ? (
                  <p className="mt-1 text-sm text-zinc-600">{user.display_name}</p>
                ) : null}
              </div>
              <div className="flex flex-wrap gap-2">
                <Badge>{formatRole(user.role)}</Badge>
                <Badge tone={user.email_verified ? "success" : "warning"}>
                  {user.email_verified ? "Verified" : "Unverified"}
                </Badge>
                {user.locked_until ? <Badge tone="danger">Locked</Badge> : null}
              </div>
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
          </Panel>

          <SessionsList userId={user.id} />
        </div>

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
  const toast = useToast();
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
      toast.notify(apiErrorMessage(_error));
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
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.userSessions(user.id) });
    },
    onError: (error) => {
      toast.notify(apiErrorMessage(error));
    },
  });

  const pending = lock.isPending || unlock.isPending || verify.isPending || revoke.isPending;
  const error = lock.error ?? unlock.error ?? verify.error ?? revoke.error;

  return (
    <aside className="h-fit rounded-lg border border-zinc-200 bg-white p-5 shadow-sm">
      <div className="flex items-start gap-3">
        <span className="grid size-9 shrink-0 place-items-center rounded-md border border-amber-200 bg-amber-50 text-amber-800">
          <ShieldAlert className="size-4" aria-hidden="true" />
        </span>
        <div className="min-w-0">
          <h2 className="text-base font-semibold">Actions</h2>
          <p className="mt-1 text-sm text-zinc-600">Confirm each action for {user.email}.</p>
        </div>
      </div>
      {error ? <StatusMessage className="mt-3">{apiErrorMessage(error)}</StatusMessage> : null}
      <div className="mt-4 grid gap-2">
        {user.locked_until ? (
          <Button onClick={() => setDialog("unlock")} variant="primary">
            Unlock user
          </Button>
        ) : (
          <Button onClick={() => setDialog("lock")} variant="danger">
            Lock user
          </Button>
        )}
        <Button
          disabled={user.email_verified}
          onClick={() => setDialog("verify")}
          variant="secondary"
        >
          Verify email
        </Button>
        <Button onClick={() => setDialog("revoke")} variant="secondary">
          Revoke sessions
        </Button>
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
        <label className="block text-sm font-medium text-zinc-800" htmlFor="lock-reason">
          Reason
          <Textarea
            className="mt-1 min-h-24"
            id="lock-reason"
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
  const toast = useToast();

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
    onError: (error, _variables, context) => {
      if (context?.previous) queryClient.setQueryData(queryKeys.user(user.id), context.previous);
      toast.notify(apiErrorMessage(error));
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
      <SkeletonLine className="h-5 w-24" />
      <div className="mt-4 grid gap-6 lg:grid-cols-[1fr_320px]">
        <SkeletonLine className="h-72 rounded-lg" />
        <SkeletonLine className="h-64 rounded-lg" />
      </div>
    </main>
  );
}
