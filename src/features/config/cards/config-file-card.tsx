import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Separator } from "@/components/ui/separator";

type ConfigFileCardProps = {
  configPath: string;
  savedAt: string;
  isDirty: boolean;
  onReset: () => void;
};

export function ConfigFileCard({
  configPath,
  savedAt,
  isDirty,
  onReset,
}: ConfigFileCardProps) {
  return (
    <Card data-slot="config-file-card">
      <CardHeader>
        <CardTitle>Config File</CardTitle>
        <CardDescription>Disk location and maintenance actions.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-1">
          <p className="text-xs uppercase tracking-[0.2em] text-muted-foreground">Location</p>
          <p className="font-mono text-xs text-foreground/80">{configPath || "--"}</p>
        </div>
        <Separator />
        <div className="space-y-1 text-sm text-muted-foreground">
          <p>Use the toolbar to save changes back to the JSONC file.</p>
          <p>Saving triggers an automatic proxy reload (and safe restart if needed).</p>
        </div>
        {savedAt ? (
          <div className="text-xs text-muted-foreground">
            Last saved at <span className="text-foreground">{savedAt}</span>
          </div>
        ) : null}
        {isDirty ? (
          <div className="rounded-md border border-border/60 bg-background/60 p-3 text-xs text-muted-foreground">
            You have unsaved changes. Reload is disabled to avoid overwriting your edits.
          </div>
        ) : null}
      </CardContent>
      <CardFooter>
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button type="button" variant="outline" disabled={!isDirty}>
              Discard changes
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Discard unsaved changes?</AlertDialogTitle>
              <AlertDialogDescription>
                This will restore the form to the last loaded config file values.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction type="button" onClick={onReset}>
                Discard
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardFooter>
    </Card>
  );
}
