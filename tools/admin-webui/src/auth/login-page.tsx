import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { api } from "@/api/client";
import { queryKeys } from "@/api/queries";
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
      form.setError("root", {
        message: apiErrorMessage(error),
      });
    },
  });

  return (
    <main className="grid min-h-dvh place-items-center bg-stone-50 px-4 text-zinc-950">
      <section className="w-full max-w-sm rounded-lg border border-zinc-200 bg-white p-6 shadow-sm">
        <h1 className="text-xl font-semibold">Admin login</h1>
        <p className="mt-2 text-sm text-zinc-600">Sign in with an admin account.</p>
        <form
          className="mt-6 space-y-4"
          onSubmit={form.handleSubmit((values) => login.mutate(values))}
        >
          <label className="block text-sm font-medium text-zinc-800">
            Email
            <input
              autoComplete="email"
              className="mt-1 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900"
              type="email"
              {...form.register("email")}
            />
          </label>
          {form.formState.errors.email ? (
            <p className="text-sm text-red-700">{form.formState.errors.email.message}</p>
          ) : null}

          <label className="block text-sm font-medium text-zinc-800">
            Password
            <input
              autoComplete="current-password"
              className="mt-1 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900"
              type="password"
              {...form.register("password")}
            />
          </label>
          {form.formState.errors.password ? (
            <p className="text-sm text-red-700">{form.formState.errors.password.message}</p>
          ) : null}

          {form.formState.errors.root ? (
            <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-800">
              {form.formState.errors.root.message}
            </p>
          ) : null}

          <button
            className="w-full rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
            disabled={login.isPending}
            type="submit"
          >
            {login.isPending ? "Signing in..." : "Sign in"}
          </button>
        </form>
      </section>
    </main>
  );
}

function safeRedirect(redirect: string | undefined): "/users" | "/auth-events" {
  if (redirect === "/admin/auth-events" || redirect === "/auth-events") return "/auth-events";
  return "/users";
}
