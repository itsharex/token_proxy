import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Label } from "@/components/ui/label";
import { UPSTREAM_COLUMNS } from "@/features/config/cards/upstreams/constants";
import type { ColumnVisibility, UpstreamColumnId } from "@/features/config/cards/upstreams/types";

type ColumnsDialogProps = {
  open: boolean;
  visibility: ColumnVisibility;
  onOpenChange: (open: boolean) => void;
  onToggleColumn: (columnId: UpstreamColumnId) => void;
};

export function ColumnsDialog({ open, visibility, onOpenChange, onToggleColumn }: ColumnsDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Visible columns</AlertDialogTitle>
          <AlertDialogDescription>Select which fields to display in the table.</AlertDialogDescription>
        </AlertDialogHeader>
        <div className="grid gap-3">
          {UPSTREAM_COLUMNS.map((column) => {
            const checkboxId = `upstream-column-${column.id}`;
            return (
              <div key={column.id} className="flex items-center gap-2">
                <input
                  id={checkboxId}
                  type="checkbox"
                  checked={visibility[column.id]}
                  onChange={() => onToggleColumn(column.id)}
                  className="size-4 rounded border-border/70 bg-background shadow-sm"
                />
                <Label htmlFor={checkboxId}>{column.label}</Label>
              </div>
            );
          })}
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel>Close</AlertDialogCancel>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

