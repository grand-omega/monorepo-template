import { format, formatDistanceToNow } from "date-fns";

export function formatRole(role: string): string {
  return role.replaceAll("_", " ");
}

export function formatOptionalDateTime(value: string | null | undefined): string {
  if (!value) return "Never";
  return formatDateTime(value);
}

export function formatDateTime(value: string): string {
  return format(new Date(value), "PP p");
}

export function formatRelativeTime(value: string): string {
  return formatDistanceToNow(new Date(value), { addSuffix: true });
}
