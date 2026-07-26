import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const attachmentVariants = cva(
  "group/attachment relative flex w-full min-w-0 items-center gap-2 rounded-2xl border border-[var(--border)] bg-[var(--surface)] text-[var(--text)] transition-colors focus-within:ring-1 focus-within:ring-[var(--text)]/30 data-[state=error]:border-[var(--danger)]/45 data-[state=error]:bg-[var(--danger-soft)] data-[state=idle]:border-dashed data-[state=idle]:border-[var(--border-strong)]",
  {
    variants: {
      size: {
        default: "px-2.5 py-2 text-sm",
        sm: "px-2 py-1.5 text-xs",
      },
      orientation: {
        horizontal: "flex-row",
        vertical: "flex-col",
      },
    },
    defaultVariants: {
      size: "default",
      orientation: "horizontal",
    },
  },
);

type AttachmentState = "idle" | "uploading" | "processing" | "error" | "done";

function Attachment({
  className,
  state = "done",
  size = "default",
  orientation = "horizontal",
  ...props
}: React.ComponentProps<"div"> &
  VariantProps<typeof attachmentVariants> & {
    state?: AttachmentState;
  }) {
  return (
    <div
      data-slot="attachment"
      data-state={state}
      data-size={size}
      data-orientation={orientation}
      className={cn(attachmentVariants({ size, orientation }), className)}
      {...props}
    />
  );
}

function AttachmentMedia({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="attachment-media"
      className={cn(
        "relative flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-[var(--surface-muted)] text-[var(--text)] group-data-[state=error]/attachment:bg-[var(--danger-soft)] group-data-[state=error]/attachment:text-[var(--danger)] [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4",
        className,
      )}
      {...props}
    />
  );
}

function AttachmentContent({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="attachment-content"
      className={cn("min-w-0 flex-1 flex flex-col gap-0.5", className)}
      {...props}
    />
  );
}

function AttachmentTitle({
  className,
  ...props
}: React.ComponentProps<"span">) {
  return (
    <span
      data-slot="attachment-title"
      className={cn(
        "truncate font-medium text-[var(--text)] group-data-[state=uploading]/attachment:animate-pulse group-data-[state=processing]/attachment:animate-pulse",
        className,
      )}
      {...props}
    />
  );
}

function AttachmentDescription({
  className,
  ...props
}: React.ComponentProps<"span">) {
  return (
    <span
      data-slot="attachment-description"
      className={cn(
        "truncate text-xs text-[var(--text-muted)] group-data-[state=error]/attachment:text-[var(--danger)]",
        className,
      )}
      {...props}
    />
  );
}

function AttachmentActions({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="attachment-actions"
      className={cn("ml-auto flex shrink-0 items-center gap-0.5", className)}
      {...props}
    />
  );
}

function AttachmentAction({
  className,
  ...props
}: React.ComponentProps<"button">) {
  return (
    <button
      type="button"
      data-slot="attachment-action"
      className={cn(
        "inline-flex size-7 items-center justify-center rounded-md text-[var(--text-muted)] transition-colors hover:bg-[var(--surface-muted)] hover:text-[var(--text)] disabled:pointer-events-none disabled:opacity-40 [&_svg]:size-3.5",
        className,
      )}
      {...props}
    />
  );
}

export {
  Attachment,
  AttachmentMedia,
  AttachmentContent,
  AttachmentTitle,
  AttachmentDescription,
  AttachmentActions,
  AttachmentAction,
  type AttachmentState,
};
