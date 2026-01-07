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

type DeleteUpstreamDialogProps = {
  dialog: DeleteDialogState;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
};

export function DeleteUpstreamDialog({ dialog, onOpenChange, onConfirm }: DeleteUpstreamDialogProps) {
  const description = dialog.open ? `${getUpstreamLabel(dialog.index)} will be removed.` : "";
  return (
    <AlertDialog open={dialog.open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete upstream?</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            className="bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

