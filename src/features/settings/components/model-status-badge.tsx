import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";

export function ModelStatusBadge({
  isLoading,
  isDownloaded,
  isDownloading,
  hasError,
}: {
  isLoading: boolean;
  isDownloaded: boolean;
  isDownloading: boolean;
  hasError: boolean;
}) {
  if (hasError) {
    return (
      <Badge className="text-[10px]" variant="destructive">
        Error
      </Badge>
    );
  }

  if (isLoading) {
    return <Spinner className="size-3" />;
  }

  if (isDownloading) {
    return null;
  }

  if (isDownloaded) {
    return (
      <Badge className="text-[10px]" variant="secondary">
        Downloaded
      </Badge>
    );
  }

  return null;
}
