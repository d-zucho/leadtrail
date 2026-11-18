import type { InputHTMLAttributes, TextareaHTMLAttributes } from "react";

const BASE =
  "w-full rounded-md border border-line bg-card px-3 py-2 text-sm outline-none focus:border-forest";

export function Input({ className = "", ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...rest} className={`${BASE} ${className}`} />;
}

export function Textarea({
  className = "",
  ...rest
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...rest} className={`${BASE} resize-none ${className}`} />;
}

export function Label({ children }: { children: React.ReactNode }) {
  return (
    <label className="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-muted">
      {children}
    </label>
  );
}
