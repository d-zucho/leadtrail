import { NavLink } from "react-router";
import { useLeads } from "@/lib/queries";

const NAV = [
  { to: "/", label: "Research desk" },
  { to: "/cases", label: "Case files" },
  { to: "/inbox", label: "Lead inbox" },
  { to: "/sources", label: "Source library" },
  { to: "/connections", label: "Connections" },
  { to: "/publication", label: "Publication queue" },
  { to: "/settings", label: "Settings" },
];

export function Sidebar() {
  const { data: leads } = useLeads();
  const openLeads = (leads ?? []).filter(
    (l) => l.status === "new" || l.status === "to_review",
  ).length;

  return (
    <aside className="flex w-56 shrink-0 flex-col bg-forest-deep px-4 py-6 text-white/80">
      <div className="mb-8 px-2">
        <div className="font-display text-3xl text-white">Fieldnotes.</div>
        <div className="mt-1 font-mono text-[10px] tracking-widest text-white/50">
          LOCAL-FIRST CASE FILES
        </div>
      </div>

      <div className="mb-2 px-2 font-mono text-[10px] tracking-widest text-white/40">
        WORKSPACE
      </div>
      <nav className="flex flex-col gap-1">
        {NAV.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) =>
              `flex items-center rounded-md px-3 py-2 text-sm transition-colors ${
                isActive
                  ? "bg-forest font-medium text-white"
                  : "hover:bg-white/5"
              }`
            }
          >
            {item.label}
            {item.to === "/inbox" && openLeads > 0 && (
              <span className="ml-auto font-mono text-[11px] opacity-60">{openLeads}</span>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="mt-auto rounded-lg bg-white/5 p-3 text-xs leading-relaxed text-white/60">
        <div className="mb-1 text-white/80">Off the record. On your terms.</div>
        Your research stays on this device until you decide otherwise.
      </div>
    </aside>
  );
}
