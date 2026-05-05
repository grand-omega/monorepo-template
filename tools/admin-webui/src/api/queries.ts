import { queryOptions } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ApiError } from "@/lib/errors";

export const queryKeys = {
  me: ["me"] as const,
};

export const meQueryOptions = queryOptions({
  queryKey: queryKeys.me,
  queryFn: async () => {
    const { data, error, response } = await api.GET("/me");
    if (error) throw new ApiError(error, response);
    return data;
  },
});
