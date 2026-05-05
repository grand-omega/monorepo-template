import { formatDateTime, formatOptionalDateTime, formatRelativeTime } from "@/lib/format";

interface TimeProps {
  value: string | null | undefined;
}

export function Time({ value }: TimeProps) {
  if (!value) return <span className="text-zinc-500">{formatOptionalDateTime(value)}</span>;

  return (
    <time dateTime={value} title={formatDateTime(value)}>
      {formatRelativeTime(value)}
    </time>
  );
}
