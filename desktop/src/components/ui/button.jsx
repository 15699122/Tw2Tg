import React from "react";
import { cn } from "../../lib/utils";

const variants = {
  default: "ui-button ui-button-default",
  secondary: "ui-button ui-button-secondary",
  ghost: "ui-button ui-button-ghost",
  destructive: "ui-button ui-button-destructive",
  outline: "ui-button ui-button-outline",
};

const sizes = {
  default: "ui-button-default-size",
  sm: "ui-button-sm",
  lg: "ui-button-lg",
  icon: "ui-button-icon",
};

export function Button({ className, variant = "default", size = "default", ...props }) {
  return <button className={cn(variants[variant], sizes[size], className)} {...props} />;
}