import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/login")({
  component: LoginPage,
});

function LoginPage() {
  return (
    <main className="grid min-h-dvh place-items-center bg-stone-50 px-4 text-zinc-950">
      <section className="w-full max-w-sm rounded-lg border border-zinc-200 bg-white p-6 shadow-sm">
        <h1 className="text-xl font-semibold">Admin login</h1>
        <p className="mt-2 text-sm text-zinc-600">
          The route is ready. The working login form is scheduled for the next roadmap step.
        </p>
        <form className="mt-6 space-y-4">
          <label className="block text-sm font-medium text-zinc-800">
            Email
            <input
              className="mt-1 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900"
              disabled
              name="email"
              type="email"
            />
          </label>
          <label className="block text-sm font-medium text-zinc-800">
            Password
            <input
              className="mt-1 w-full rounded-md border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-900"
              disabled
              name="password"
              type="password"
            />
          </label>
          <button
            className="w-full rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
            disabled
            type="button"
          >
            Sign in
          </button>
        </form>
      </section>
    </main>
  );
}
