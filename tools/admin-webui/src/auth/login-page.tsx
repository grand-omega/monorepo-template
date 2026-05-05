import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { LockKeyhole, KeyRound } from "lucide-react";
import { z } from "zod";
import { queryKeys } from "@/api/queries";
import { useToast } from "@/components/toast";
import { Button, Input, Panel, StatusMessage } from "@/components/ui";
import { queryClient } from "@/router";
import { apiErrorMessage } from "@/lib/errors";
import { completeWebauthnLogin, loginWithPassword, type LoginOutcome } from "@/auth/webauthn";

const loginSchema = z.object({
  email: z.string().trim().email("Enter a valid email address"),
  password: z.string().min(1, "Enter your password"),
});

type LoginFormValues = z.infer<typeof loginSchema>;

interface LoginSearch {
  redirect?: string;
}

interface PendingWebauthn {
  pending_token: string;
  challenge: Extract<LoginOutcome, { kind: "webauthn_required" }>["challenge"];
}

export function LoginPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const search = useSearch({ from: "/login" }) as LoginSearch;
  const [pending, setPending] = useState<PendingWebauthn | null>(null);
  const form = useForm<LoginFormValues>({
    resolver: zodResolver(loginSchema),
    defaultValues: {
      email: "",
      password: "",
    },
  });

  const goAfterLogin = async () => {
    await queryClient.invalidateQueries({ queryKey: queryKeys.me });
    if (safeRedirect(search.redirect) === "/auth-events") {
      await navigate({
        to: "/auth-events",
        search: { cursor: undefined, event_type: undefined, limit: 100, user_id: undefined },
      });
      return;
    }
    await navigate({ to: "/users", search: { q: undefined, cursor: undefined, limit: 50 } });
  };

  const login = useMutation({
    mutationFn: (body: LoginFormValues) => loginWithPassword(body.email, body.password),
    onSuccess: async (outcome) => {
      if (outcome.kind === "webauthn_required") {
        setPending({ pending_token: outcome.pending_token, challenge: outcome.challenge });
        return;
      }
      await goAfterLogin();
    },
    onError: (error) => {
      toast.notify(apiErrorMessage(error));
      form.setError("root", { message: apiErrorMessage(error) });
    },
  });

  const finishWebauthn = useMutation({
    mutationFn: async () => {
      if (!pending) throw new Error("no pending login");
      return completeWebauthnLogin(pending.pending_token, pending.challenge);
    },
    onSuccess: async () => {
      setPending(null);
      await goAfterLogin();
    },
    onError: (error) => {
      toast.notify(apiErrorMessage(error));
      form.setError("root", { message: apiErrorMessage(error) });
    },
  });

  if (pending) {
    return (
      <main className="grid min-h-dvh place-items-center bg-zinc-50 px-4 text-zinc-950">
        <Panel className="w-full max-w-sm p-6">
          <div className="flex items-center gap-3">
            <span className="grid size-9 place-items-center rounded-md border border-zinc-200 bg-zinc-100 text-zinc-700">
              <KeyRound className="size-4" aria-hidden="true" />
            </span>
            <div>
              <h1 className="text-xl font-semibold">Confirm with passkey</h1>
              <p className="mt-1 text-sm text-zinc-600">
                Use Touch ID, Windows Hello, or your security key to finish signing in.
              </p>
            </div>
          </div>
          {form.formState.errors.root ? (
            <div className="mt-4">
              <StatusMessage>{form.formState.errors.root.message}</StatusMessage>
            </div>
          ) : null}
          <div className="mt-6 flex flex-col gap-2">
            <Button
              className="w-full"
              disabled={finishWebauthn.isPending}
              onClick={() => finishWebauthn.mutate()}
              type="button"
              variant="primary"
            >
              {finishWebauthn.isPending ? "Waiting for passkey..." : "Use passkey"}
            </Button>
            <Button
              className="w-full"
              onClick={() => {
                setPending(null);
                form.clearErrors("root");
              }}
              type="button"
              variant="ghost"
            >
              Cancel
            </Button>
          </div>
        </Panel>
      </main>
    );
  }

  return (
    <main className="grid min-h-dvh place-items-center bg-zinc-50 px-4 text-zinc-950">
      <Panel className="w-full max-w-sm p-6">
        <div className="flex items-center gap-3">
          <span className="grid size-9 place-items-center rounded-md border border-zinc-200 bg-zinc-100 text-zinc-700">
            <LockKeyhole className="size-4" aria-hidden="true" />
          </span>
          <div>
            <h1 className="text-xl font-semibold">Admin login</h1>
            <p className="mt-1 text-sm text-zinc-600">Sign in with an admin account.</p>
          </div>
        </div>
        <form
          className="mt-6 space-y-4"
          onSubmit={form.handleSubmit((values) => login.mutate(values))}
        >
          <label className="block text-sm font-medium text-zinc-800" htmlFor="login-email">
            Email
            <Input
              autoComplete="email"
              className="mt-1"
              data-search-input="true"
              id="login-email"
              type="email"
              {...form.register("email")}
            />
          </label>
          {form.formState.errors.email ? (
            <p className="text-sm text-red-700">{form.formState.errors.email.message}</p>
          ) : null}

          <label className="block text-sm font-medium text-zinc-800" htmlFor="login-password">
            Password
            <Input
              autoComplete="current-password"
              className="mt-1"
              id="login-password"
              type="password"
              {...form.register("password")}
            />
          </label>
          {form.formState.errors.password ? (
            <p className="text-sm text-red-700">{form.formState.errors.password.message}</p>
          ) : null}

          {form.formState.errors.root ? (
            <StatusMessage>{form.formState.errors.root.message}</StatusMessage>
          ) : null}

          <Button className="w-full" disabled={login.isPending} type="submit" variant="primary">
            {login.isPending ? "Signing in..." : "Sign in"}
          </Button>
        </form>
      </Panel>
    </main>
  );
}

function safeRedirect(redirect: string | undefined): "/users" | "/auth-events" {
  if (redirect === "/admin/auth-events" || redirect === "/auth-events") return "/auth-events";
  return "/users";
}
