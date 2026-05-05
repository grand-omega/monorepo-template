import { Link, Outlet, createFileRoute, redirect } from "@tanstack/react-router";
import { useMutation } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { KeyRound, LogOut, ShieldCheck, Users } from "lucide-react";
import { api } from "@/api/client";
import { useToast } from "@/components/toast";
import { meQueryOptions } from "@/api/queries";
import { ApiError, apiErrorMessage } from "@/lib/errors";
import { Button, SkeletonLine, buttonClassName } from "@/components/ui";

export const Route = createFileRoute("/_authenticated")({
  beforeLoad: async ({ context, location }) => {
    try {
      const session = await context.queryClient.fetchQuery(meQueryOptions);

      return { session };
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        throw redirect({
          to: "/login",
          search: { redirect: location.href },
        });
      }
      throw error;
    }
  },
  component: AuthenticatedLayout,
  pendingComponent: AuthenticatedSkeleton,
});

function AuthenticatedLayout() {
  const { session } = Route.useRouteContext();
  const { queryClient } = Route.useRouteContext();
  const navigate = Route.useNavigate();
  const toast = useToast();
  const awaitingShortcut = useRef(false);

  useEffect(() => {
    function isTypingTarget(target: EventTarget | null) {
      if (!(target instanceof HTMLElement)) return false;
      return Boolean(target.closest("input, textarea, select, [contenteditable='true']"));
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;

      if (event.key === "Escape" && isTypingTarget(event.target)) {
        (event.target as HTMLElement).blur();
        return;
      }

      if (event.key === "/" && !isTypingTarget(event.target)) {
        const input = document.querySelector<HTMLInputElement>("[data-search-input='true']");
        if (input) {
          event.preventDefault();
          input.focus();
          input.select();
        }
        return;
      }

      if (isTypingTarget(event.target)) return;

      if (event.key === "g") {
        awaitingShortcut.current = true;
        window.setTimeout(() => {
          awaitingShortcut.current = false;
        }, 1000);
        return;
      }

      if (!awaitingShortcut.current) return;
      awaitingShortcut.current = false;

      if (event.key === "u") {
        event.preventDefault();
        void navigate({ to: "/users", search: { q: undefined, cursor: undefined, limit: 50 } });
      }

      if (event.key === "e") {
        event.preventDefault();
        void navigate({
          to: "/auth-events",
          search: { cursor: undefined, event_type: undefined, limit: 100, user_id: undefined },
        });
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [navigate]);

  const logout = useMutation({
    mutationFn: async () => {
      const { error, response } = await api.POST("/logout");
      if (error) throw new ApiError(error, response);
    },
    onSuccess: async () => {
      queryClient.clear();
      await navigate({ to: "/login", search: { redirect: undefined } });
    },
    onError: (error) => {
      toast.notify(apiErrorMessage(error));
    },
  });

  return (
    <div className="min-h-dvh bg-zinc-50 text-zinc-950">
      <header className="sticky top-0 z-40 border-b border-zinc-200 bg-white/95 backdrop-blur">
        <div className="mx-auto flex max-w-6xl flex-col gap-3 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <p className="text-sm font-semibold tracking-normal">lab-rust-server admin</p>
            <p className="truncate text-xs text-zinc-500">{session.email}</p>
          </div>
          <nav aria-label="Admin navigation" className="flex flex-wrap items-center gap-2">
            <Link
              activeProps={{
                className: "border-zinc-950 bg-zinc-950 text-white hover:bg-zinc-900",
              }}
              className={buttonClassName({ variant: "ghost" })}
              search={{ q: undefined, cursor: undefined, limit: 50 }}
              to="/users"
            >
              <Users className="size-4" aria-hidden="true" />
              Users
            </Link>
            <Link
              activeProps={{
                className: "border-zinc-950 bg-zinc-950 text-white hover:bg-zinc-900",
              }}
              className={buttonClassName({ variant: "ghost" })}
              search={{ cursor: undefined, event_type: undefined, limit: 100, user_id: undefined }}
              to="/auth-events"
            >
              <ShieldCheck className="size-4" aria-hidden="true" />
              Auth events
            </Link>
            <Link
              activeProps={{
                className: "border-zinc-950 bg-zinc-950 text-white hover:bg-zinc-900",
              }}
              className={buttonClassName({ variant: "ghost" })}
              to="/passkeys"
            >
              <KeyRound className="size-4" aria-hidden="true" />
              Passkeys
            </Link>
            <Button disabled={logout.isPending} onClick={() => logout.mutate()} variant="secondary">
              <LogOut className="size-4" aria-hidden="true" />
              {logout.isPending ? "Signing out..." : "Sign out"}
            </Button>
          </nav>
        </div>
        {logout.error ? (
          <p className="mx-auto max-w-6xl px-4 pb-3 text-sm text-red-700" role="alert">
            {apiErrorMessage(logout.error)}
          </p>
        ) : null}
      </header>
      <Outlet />
    </div>
  );
}

function AuthenticatedSkeleton() {
  return (
    <main className="min-h-dvh bg-zinc-50 px-4 py-6">
      <div className="mx-auto max-w-6xl space-y-4">
        <SkeletonLine className="h-10 w-64" />
        <SkeletonLine className="h-40 rounded-lg" />
      </div>
    </main>
  );
}
