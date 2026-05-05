import { queryOptions } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ApiError } from "@/lib/errors";

export const queryKeys = {
  me: ["me"] as const,
  user: (id: string) => ["users", id] as const,
  userSessions: (id: string) => ["users", id, "sessions"] as const,
};

export const meQueryOptions = queryOptions({
  queryKey: queryKeys.me,
  queryFn: async () => {
    const { data, error, response } = await api.GET("/me");
    if (error) throw new ApiError(error, response);
    if (!response.ok) throw new Error(`Request failed with status ${response.status}`);
    if (!data) throw new Error("Current admin session response was empty");
    return data;
  },
});
