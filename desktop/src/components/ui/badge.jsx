import React from "react";
import { cn } from "../../lib/utils";

const variants = {
  default: "ui-badge ui-badge-default",
  secondary: "ui-badge ui-badge-secondary",
  outline: "ui-badge ui-badge-outline",
  success: "ui-badge ui-badge-success",
  warning: "ui-badge ui-badge-warning",
  destructive: "ui-badge ui-badge-destructive",
};

export function Badge({ className, variant = "default", ...props }) {
  return <span className={cn(variants[variant], className)} {...props} />;
}