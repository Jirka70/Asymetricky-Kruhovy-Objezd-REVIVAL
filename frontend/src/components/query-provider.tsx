"use client";
import type { ReactNode } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { createQueryClient } from "@/lib/api";

let browserClient: QueryClient | undefined;
export default function QueryProvider({ children }: { children: ReactNode }) {
  const client = typeof window === "undefined"
    ? createQueryClient()
    : (browserClient ??= createQueryClient());
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
