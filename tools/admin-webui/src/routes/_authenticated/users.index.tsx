import { createFileRoute } from "@tanstack/react-router";
import { UsersListPage } from "@/features/users/list-page";

export const Route = createFileRoute("/_authenticated/users/")({
  component: UsersListPage,
});
