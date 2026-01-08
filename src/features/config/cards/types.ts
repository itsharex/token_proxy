export type StatusBadgeId = "working" | "error" | "unsaved" | "saved" | "idle";

export type StatusBadge = {
  id: StatusBadgeId;
  label: string;
  variant: "default" | "secondary" | "destructive" | "outline";
};
