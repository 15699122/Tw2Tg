import React from "react";
import { cn } from "../../lib/utils";

export function Separator({ className, ...props }) {
  return <div role="separator" className={cn("ui-separator", className)} {...props} />;
}