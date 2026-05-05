import type { components } from "@/api/schema";

export type ErrorBody = components["schemas"]["ErrorBody"];

export class ApiError extends Error {
  readonly code: string;
  readonly requestId: string | null;
  readonly status: number;

  constructor(error: ErrorBody, response: Response) {
    const requestId = response.headers.get("x-request-id");
    super(requestId ? `${error.message} (request ${requestId})` : error.message);
    this.name = "ApiError";
    this.code = error.code;
    this.requestId = requestId;
    this.status = response.status;
  }
}

export function apiErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return "Request failed";
}
