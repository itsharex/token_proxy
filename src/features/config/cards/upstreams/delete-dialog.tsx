import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { getUpstreamLabel } from "@/features/config/cards/upstreams/constants";
import type { DeleteDialogState } from "@/features/config/cards/upstreams/types";
import { m } from "@/paraglide/messages.js";

type DeleteUpstreamDialogProps = {
  dialog: DeleteDialogState;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
};

export function DeleteUpstreamDialog({ dialog, onOpenChange, onConfirm }: DeleteUpstreamDialogProps) {
  const description = dialog.open
    ? m.upstreams_delete_description({ rowLabel: getUpstreamLabel(dialog.index) })
    : "";
  return (
    <AlertDialog open={dialog.open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{m.upstreams_delete_title()}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{m.common_cancel()}</AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            className="bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60"
          >
            {m.common_delete()}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
