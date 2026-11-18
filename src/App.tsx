import { lazy, Suspense } from "react";
import { HashRouter, Route, Routes } from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "@/components/layout/AppShell";
import { CaseDetail } from "@/routes/CaseDetail";
import { Cases } from "@/routes/Cases";
import { Dashboard } from "@/routes/Dashboard";
import { Inbox } from "@/routes/Inbox";
import { Publication } from "@/routes/Publication";
import { Sources } from "@/routes/Sources";

// The editor (TipTap) and the graph (React Flow) are large, so load them on demand.
const DocumentEditor = lazy(() =>
  import("@/routes/DocumentEditor").then((m) => ({ default: m.DocumentEditor })),
);
const Connections = lazy(() =>
  import("@/routes/Connections").then((m) => ({ default: m.Connections })),
);

const queryClient = new QueryClient();

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <HashRouter>
        <Routes>
          <Route element={<AppShell />}>
            <Route index element={<Dashboard />} />
            <Route path="cases" element={<Cases />} />
            <Route path="cases/:id" element={<CaseDetail />} />
            <Route
              path="cases/:id/docs/:docId"
              element={
                <Suspense fallback={<p className="text-muted">Loading…</p>}>
                  <DocumentEditor />
                </Suspense>
              }
            />
            <Route path="inbox" element={<Inbox />} />
            <Route path="sources" element={<Sources />} />
            <Route
              path="connections"
              element={
                <Suspense fallback={<p className="text-muted">Loading…</p>}>
                  <Connections />
                </Suspense>
              }
            />
            <Route path="publication" element={<Publication />} />
          </Route>
        </Routes>
      </HashRouter>
    </QueryClientProvider>
  );
}
