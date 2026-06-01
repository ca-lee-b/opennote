import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function ErrorView({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <main
      className={cn(
        "flex min-h-svh items-center justify-center bg-background px-4 py-8 text-left sm:px-6",
        className
      )}
    >
      <div className="w-full max-w-lg rounded-xl border border-border bg-card p-8 text-foreground shadow-none sm:p-10">
        {children}
      </div>
    </main>
  );
}

export function ErrorHeader({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <h1
      className={cn(
        "mt-4 font-heading font-semibold text-3xl text-foreground tracking-tight sm:text-4xl",
        className
      )}
    >
      {children}
    </h1>
  );
}

export function ErrorDescription({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <p
      className={cn(
        "mt-6 text-base text-muted-foreground leading-7",
        className
      )}
    >
      {children}
    </p>
  );
}

export function ErrorActions({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("mt-8 flex flex-wrap items-center gap-3", className)}>
      {children}
    </div>
  );
}
