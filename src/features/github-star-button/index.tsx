import { Star } from "lucide-react";
import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export default function GithubStarButton() {
  return (
    <a
      className={cn(buttonVariants({ size: "sm" }), "inline-flex")}
      href="https://github.com/mrlightful/create-tauri-react"
      rel="noreferrer"
      target="_blank"
    >
      <Star className="mr-1" size={16} /> Star Github
    </a>
  );
}
