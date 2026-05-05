import { useQuery } from "@tanstack/react-query";
import { meQueryOptions } from "@/api/queries";

export function useSession() {
  return useQuery(meQueryOptions);
}
