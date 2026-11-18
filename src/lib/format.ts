/** Today as YYYY-MM-DD in local time, to compare against stored due dates. */
export function todayISO(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** "22 Nov" for a YYYY-MM-DD string. Parsed manually to avoid UTC shifting the day. */
export function formatDue(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

export function greeting(now = new Date()): string {
  const h = now.getHours();
  if (h < 5) return "Working late";
  if (h < 12) return "Good morning";
  if (h < 18) return "Good afternoon";
  return "Good evening";
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 ** 3) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 ** 3).toFixed(2)} GB`;
}

/** SQLite `datetime('now')` is UTC without a zone marker. */
export function parseSqlite(sqliteUtc: string): Date {
  return new Date(sqliteUtc.replace(" ", "T") + "Z");
}

export function isToday(sqliteUtc: string): boolean {
  return parseSqlite(sqliteUtc).toDateString() === new Date().toDateString();
}

export function withinHours(sqliteUtc: string, hours: number): boolean {
  return Date.now() - parseSqlite(sqliteUtc).getTime() < hours * 3600 * 1000;
}

export function timeAgo(sqliteUtc: string): string {
  const then = parseSqlite(sqliteUtc).getTime();
  const secs = Math.max(0, Math.round((Date.now() - then) / 1000));
  if (secs < 60) return "just now";
  const mins = Math.round(secs / 60);
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours} hr ago`;
  const days = Math.round(hours / 24);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}
