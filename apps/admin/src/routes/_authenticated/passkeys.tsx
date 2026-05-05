import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { KeyRound, Trash2 } from "lucide-react";
import { useState } from "react";
import { useToast } from "@/components/toast";
import {
  Badge,
  Button,
  EmptyState,
  Input,
  PageHeader,
  Panel,
  StatusMessage,
  Table,
  TableCell,
  TableHead,
  TableHeaderCell,
} from "@/components/ui";
import { apiErrorMessage } from "@/lib/errors";
import {
  deleteCredential,
  listCredentials,
  registerNewCredential,
  type CredentialSummary,
} from "@/auth/webauthn";

const queryKey = ["webauthn", "credentials"] as const;

export const Route = createFileRoute("/_authenticated/passkeys")({
  component: PasskeysPage,
});

function PasskeysPage() {
  const queryClient = useQueryClient();
  const toast = useToast();
  const [label, setLabel] = useState("");

  const credentials = useQuery({
    queryKey,
    queryFn: listCredentials,
  });

  const enroll = useMutation({
    mutationFn: () => registerNewCredential(label.trim()),
    onSuccess: async (created) => {
      setLabel("");
      toast.notify(`Registered "${created.label}"`);
      await queryClient.invalidateQueries({ queryKey });
    },
    onError: (error) => {
      toast.notify(apiErrorMessage(error));
    },
  });

  const remove = useMutation({
    mutationFn: (id: string) => deleteCredential(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey });
    },
    onError: (error) => {
      toast.notify(apiErrorMessage(error));
    },
  });

  const items = credentials.data?.items ?? [];

  return (
    <main className="mx-auto max-w-4xl space-y-6 px-4 py-6">
      <PageHeader
        title="Passkeys"
        description="Add a passkey for two-step admin login. Once any passkey is registered, password alone is no longer sufficient — keep at least two devices enrolled to avoid lockout."
      />

      <Panel className="p-5">
        <div className="flex items-center gap-3">
          <span className="grid size-8 place-items-center rounded-md border border-zinc-200 bg-zinc-100 text-zinc-700">
            <KeyRound className="size-4" aria-hidden="true" />
          </span>
          <h2 className="text-base font-semibold text-zinc-950">Add a passkey</h2>
        </div>
        <form
          className="mt-4 flex flex-col gap-3 sm:flex-row sm:items-end"
          onSubmit={(e) => {
            e.preventDefault();
            if (!label.trim()) return;
            enroll.mutate();
          }}
        >
          <label className="flex-1 text-sm font-medium text-zinc-800" htmlFor="passkey-label">
            Label
            <Input
              className="mt-1"
              id="passkey-label"
              maxLength={80}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="e.g. MacBook Touch ID"
              value={label}
            />
          </label>
          <Button disabled={enroll.isPending || !label.trim()} type="submit" variant="primary">
            {enroll.isPending ? "Waiting for authenticator..." : "Register passkey"}
          </Button>
        </form>
        {enroll.error ? (
          <StatusMessage className="mt-3">{apiErrorMessage(enroll.error)}</StatusMessage>
        ) : null}
      </Panel>

      <Panel>
        {credentials.isLoading ? (
          <EmptyState title="Loading…" />
        ) : items.length === 0 ? (
          <EmptyState title="No passkeys registered">
            You can sign in with password only until you register one.
          </EmptyState>
        ) : (
          <Table>
            <TableHead>
              <tr>
                <TableHeaderCell>Label</TableHeaderCell>
                <TableHeaderCell>Registered</TableHeaderCell>
                <TableHeaderCell>Last used</TableHeaderCell>
                <TableHeaderCell className="pr-4 text-right">Actions</TableHeaderCell>
              </tr>
            </TableHead>
            <tbody>
              {items.map((c) => (
                <CredentialRow
                  credential={c}
                  key={c.id}
                  onDelete={() => {
                    if (
                      window.confirm(
                        `Remove passkey "${c.label}"? You won't be able to use this authenticator to sign in anymore.`
                      )
                    ) {
                      remove.mutate(c.id);
                    }
                  }}
                  removing={remove.isPending && remove.variables === c.id}
                />
              ))}
            </tbody>
          </Table>
        )}
      </Panel>
    </main>
  );
}

function CredentialRow({
  credential,
  onDelete,
  removing,
}: {
  credential: CredentialSummary;
  onDelete: () => void;
  removing: boolean;
}) {
  return (
    <tr className="border-b border-zinc-100 last:border-b-0">
      <TableCell className="font-medium">{credential.label}</TableCell>
      <TableCell className="text-zinc-600">
        {new Date(credential.created_at).toLocaleString()}
      </TableCell>
      <TableCell className="text-zinc-600">
        {credential.last_used_at ? (
          new Date(credential.last_used_at).toLocaleString()
        ) : (
          <Badge tone="neutral">Never</Badge>
        )}
      </TableCell>
      <TableCell className="pr-4 text-right">
        <Button
          aria-label={`Remove passkey ${credential.label}`}
          disabled={removing}
          onClick={onDelete}
          size="sm"
          variant="danger"
        >
          <Trash2 className="size-3.5" aria-hidden="true" />
          {removing ? "Removing..." : "Remove"}
        </Button>
      </TableCell>
    </tr>
  );
}
