import * as React from "react"
import * as LabelPrimitive from "radix-ui/label"
import { cn } from "@/lib/utils" // Ajuste selon ton projet

// Conteneur de formulaire standard
function Form({
  className,
  ...props
}: React.ComponentProps<"form">) {
  return (
    <form
      data-slot="form"
      className={cn("space-y-4", className)}
      {...props}
    />
  )
}

// Groupe pour associer un label, un input et un message d'erreur
function FormField({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="form-field"
      className={cn("space-y-2", className)}
      {...props}
    />
  )
}

// Label basé sur Radix UI
function FormLabel({
  className,
  ...props
}: React.ComponentProps<typeof LabelPrimitive.Root>) {
  return (
    <LabelPrimitive.Root
      data-slot="form-label"
      className={cn(
        "text-sm font-medium leading-none text-foreground peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
        className
      )}
      {...props}
    />
  )
}

// Input de formulaire stylisé
function FormInput({
  className,
  type,
  ...props
}: React.ComponentProps<"input">) {
  return (
    <input
      data-slot="form-input"
      type={type}
      className={cn(
        "flex h-10 w-full rounded-xl border border-input bg-background px-3 py-2 text-sm text-foreground shadow-xs transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
        className
      )}
      {...props}
    />
  )
}

// Message d'erreur (généralement alimenté par Zod / React Hook Form)
function FormMessage({
  className,
  children,
  ...props
}: React.ComponentProps<"p">) {
  return (
    <p
      data-slot="form-message"
      className={cn("text-xs font-medium text-destructive", className)}
      {...props}
    >
      {children}
    </p>
  )
}

export {
  Form,
  FormField,
  FormLabel,
  FormInput,
  FormMessage,
}