// Delivers fictional mail to a (GreenMail) SMTP server. No dependencies, Node 22.18+ (type stripping).
// Library for the specs and the CLI `node e2e/send-mail.mjs` (see there).
// Fixtures are e2e/fixtures/mail/<name>.eml; {{d+N}} becomes the UTC date N days from today
// (YYYYMMDD) and {{long+N}} the same day as "Month D, YYYY".
import { readFile } from "node:fs/promises";
import { connect } from "node:net";
import { join } from "node:path";

// Relative to the working directory (the repository root), so the module needs no `import.meta`.
const dir = join(process.cwd(), "e2e", "fixtures", "mail");
const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

function day(n: number): Date {
  return new Date(Date.now() + n * 86_400_000);
}

export function expand(text: string): string {
  return text
    .replace(/\{\{d([+-]\d+)\}\}/g, (_: string, n: string) =>
      day(Number(n)).toISOString().slice(0, 10).replaceAll("-", ""),
    )
    .replace(/\{\{long([+-]\d+)\}\}/g, (_: string, n: string) => {
      const d = day(Number(n));
      return `${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()}, ${d.getUTCFullYear()}`;
    });
}

export interface SendOpts {
  host?: string;
  port: number | string;
  to: string;
  from?: string;
}

export async function sendRaw({
  host = "127.0.0.1",
  port,
  to,
  from = "sender@example.com",
  raw,
}: SendOpts & { raw: string }): Promise<void> {
  const data = raw.replace(/\r?\n/g, "\r\n").replace(/^\./gm, "..");
  const steps: [string | null, number][] = [
    [null, 220],
    ["EHLO e2e.example.com", 250],
    [`MAIL FROM:<${from}>`, 250],
    [`RCPT TO:<${to}>`, 250],
    ["DATA", 354],
    [`${data}${data.endsWith("\r\n") ? "" : "\r\n"}.`, 250],
    ["QUIT", 221],
  ];
  await new Promise<void>((resolve, reject) => {
    const sock = connect(Number(port), host);
    let buf = "";
    let i = 0;
    sock.setTimeout(15_000, () => reject(new Error("smtp timeout")));
    sock.on("error", reject);
    sock.on("data", (chunk) => {
      buf += chunk.toString("utf8");
      // A reply is complete when its last line is "NNN text" (no dash after the code).
      const lines = buf.split("\r\n");
      const last = lines[lines.length - 2];
      if (!last || !/^\d{3} /.test(last)) return;
      const code = Number(last.slice(0, 3));
      buf = "";
      if (code !== steps[i][1])
        return reject(new Error(`smtp: expected ${steps[i][1]}, got ${last}`));
      i += 1;
      if (i === steps.length) {
        sock.end();
        return resolve();
      }
      sock.write(`${steps[i][0]}\r\n`);
    });
  });
}

export async function sendFixture(name: string, opts: SendOpts): Promise<void> {
  const raw = expand(await readFile(join(dir, `${name}.eml`), "utf8"));
  await sendRaw({ ...opts, raw });
}
