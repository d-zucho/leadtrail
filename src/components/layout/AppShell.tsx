import { useEffect } from "react";
import { Outlet } from "react-router";
import { CommandPalette } from "@/components/CommandPalette";
import { QuickCapture } from "@/components/QuickCapture";
import { Button } from "@/components/ui/Button";
import { useUi } from "@/store/ui";
import { Sidebar } from "./Sidebar";

export function AppShell() {
  const openCapture = useUi((s) => s.openCapture);
  const openPalette = useUi((s) => s.openPalette);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "j") {
        e.preventDefault();
        openCapture();
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        openPalette();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [openCapture, openPalette]);

  return (
    <div className="flex h-full">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 items-center justify-between border-b border-line px-8">
          <button
            onClick={openPalette}
            className="flex w-80 items-center justify-between rounded-md border border-line bg-card px-3 py-1.5 text-left text-sm text-muted hover:border-forest"
          >
            Search your newsroom
            <span className="font-mono text-[11px]">⌘K</span>
          </button>
          <Button variant="primary" onClick={openCapture} className="py-1.5">
            Quick capture <span className="font-mono text-[11px] opacity-70">⌘J</span>
          </Button>
        </header>
        <main className="flex-1 overflow-y-auto px-8 py-8">
          <Outlet />
        </main>
      </div>
      <QuickCapture />
      <CommandPalette />
    </div>
  );
}
