import { createFileRoute } from "@tanstack/react-router";
import { UserDetailPage } from "@/features/users/detail-page";

export const Route = createFileRoute("/_authenticated/users/$id")({
  component: UserDetailPage,
});
