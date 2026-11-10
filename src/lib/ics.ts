// Calendar file (.ics) for the calendar's Export: all-day events that any
// calendar app can import. Only what's given here goes in (the caller leaves
// people's names out).

export interface IcsEvent {
  uid: string;
  /** First day and last day (both included), as local dates. */
  start: Date;
  end: Date;
  title: string;
  description?: string;
  location?: string;
}

const pad = (n: number) => String(n).padStart(2, "0");
const day = (d: Date) => `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`;
const stamp = (d: Date) => `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}Z`;
/** Text as the format wants it: backslash, semicolon, comma and line breaks escaped. */
export const escapeText = (s: string) => s.replace(/\\/g, "\\\\").replace(/;/g, "\\;").replace(/,/g, "\\,").replace(/\r?\n/g, "\\n");

/** Long lines are folded at 75 characters (a line break and a space). */
function fold(line: string): string {
  const out: string[] = [];
  let rest = line;
  while ([...rest].length > 75) {
    const chars = [...rest];
    out.push(chars.slice(0, 75).join(""));
    rest = " " + chars.slice(75).join("");
  }
  out.push(rest);
  return out.join("\r\n");
}

export function toIcs(events: IcsEvent[], now = new Date()): string {
  const lines = ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Pregnancy OSC//Calendar//EN", "CALSCALE:GREGORIAN", "X-WR-CALNAME:Pregnancy OSC"];
  for (const e of events) {
    // All-day events end the day AFTER their last day.
    const after = new Date(e.end.getFullYear(), e.end.getMonth(), e.end.getDate() + 1);
    lines.push(
      "BEGIN:VEVENT",
      `UID:${e.uid}@pregnancy-osc`,
      `DTSTAMP:${stamp(now)}`,
      `DTSTART;VALUE=DATE:${day(e.start)}`,
      `DTEND;VALUE=DATE:${day(after)}`,
      `SUMMARY:${escapeText(e.title)}`,
      ...(e.description ? [`DESCRIPTION:${escapeText(e.description)}`] : []),
      ...(e.location ? [`LOCATION:${escapeText(e.location)}`] : []),
      "TRANSP:TRANSPARENT",
      "END:VEVENT",
    );
  }
  lines.push("END:VCALENDAR");
  return lines.map(fold).join("\r\n") + "\r\n";
}
