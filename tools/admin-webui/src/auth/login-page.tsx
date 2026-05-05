import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { LockKeyhole } from "lucide-react";
import { z } from "zod";
import { api } from "@/api/client";
import { queryKeys } from "@/api/queries";
import { useToast } from "@/components/toast";
import { Button, Input, Panel, StatusMessage } from "@/components/ui";
import { queryClient } from "@/router";
import { ApiError, apiErrorMessage } from "@/lib/errors";

const loginSchema = z.object({
  email: z.string().trim().email("Enter a valid email address"),
  password: z.string().min(1, "Enter your password"),
});

type LoginFormValues = z.infer<typeof loginSchema>;

interface LoginSearch {
  redirect?: string;
}

export function LoginPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const search = useSearch({ from: "/login" }) as LoginSearch;
  const form = useForm<LoginFormValues>({
    resolver: zodResolver(loginSchema),
    defaultValues: {
      email: "",
      password: "",
    },
  });

  const login = useMutation({
    mutationFn: async (body: LoginFormValues) => {
      const { error, response } = await api.POST("/login", { body });
      if (error) throw new ApiError(error, response);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.me });
      if (safeRedirect(search.redirect) === "/auth-events") {
        await navigate({
          to: "/auth-events",
          search: { cursor: undefined, event_type: undefined, limit: 100, user_id: undefined },
        });
        return;
      }
      await navigate({ to: "/users", search: { q: undefined, cursor: undefined, limit: 50 } });
    },
    onError: (error) => {
      toast.notify(apiErrorMessage(error));
      form.setError("root", {
        message: apiErrorMessage(error),
      });
    },
  });

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
