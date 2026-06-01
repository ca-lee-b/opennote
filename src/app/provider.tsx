import { type ReactNode, Suspense } from "react";
import { ErrorBoundary } from "react-error-boundary";
import { Spinner } from "@/components/ui/spinner";
import { TooltipProvider } from "@/components/ui/tooltip";
import AppErrorPage from "@/features/errors/app-error";

export default function AppProvider({ children }: { children: ReactNode }) {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-svh items-center justify-center bg-background">
          <Spinner className="size-5" />
        </div>
      }
    >
      <ErrorBoundary FallbackComponent={AppErrorPage}>
        <TooltipProvider>{children}</TooltipProvider>
      </ErrorBoundary>
    </Suspense>
  );
}
