// Fixture server for the e2e stack (no dependencies, Node 18+).
//   GET /ics/<name>.ics   files from e2e/fixtures/ics with {{d+N}} replaced by the UTC date
//                         (YYYYMMDD) N days from today, so the fixtures never go stale
//   GET /v1/forecast      an Open-Meteo-shaped forecast: freezing and very likely rain all the
//                         time, so "Frost tonight" and "Likely rain today" always fire
// Usage: node server.mjs <port>
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const dir = join(
    dirname(fileURLToPath(import.meta.url)),
    "..",
    "fixtures",
    "ics",
);
const port = Number(process.argv[2] ?? 18091);

const pad = (n) => String(n).padStart(2, "0");
function ymd(offsetDays) {
    const d = new Date(Date.now() + offsetDays * 86_400_000);
    return `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}`;
}

function localParts(date, tz) {
    const f = new Intl.DateTimeFormat("en-CA", {
        timeZone: tz,
        hourCycle: "h23",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
    });
    return Object.fromEntries(
        f.formatToParts(date).map((p) => [p.type, p.value]),
    );
}

function forecast(tz) {
    const start = new Date(Math.floor(Date.now() / 3_600_000) * 3_600_000);
    const hours = Array.from(
        { length: 48 },
        (_, i) => new Date(start.getTime() + i * 3_600_000),
    );
    const time = hours.map((h) => {
        const p = localParts(h, tz);
        return `${p.year}-${p.month}-${p.day}T${p.hour}:${p.minute}`;
    });
    const fill = (v) => hours.map(() => v);
    const days = Array.from({ length: 7 }, (_, i) => {
        const p = localParts(new Date(Date.now() + i * 86_400_000), tz);
        return `${p.year}-${p.month}-${p.day}`;
    });
    const now = localParts(new Date(), tz);
    return {
        latitude: 40,
        longitude: -75,
        utc_offset_seconds: 0,
        timezone: tz,
        current: {
            time: `${now.year}-${now.month}-${now.day}T${now.hour}:${now.minute}`,
            temperature_2m: -3,
            apparent_temperature: -6,
            weather_code: 61,
            wind_speed_10m: 12,
            wind_gusts_10m: 25,
            precipitation: 0.4,
            is_day: 1,
        },
        hourly: {
            time,
            temperature_2m: fill(-3),
            apparent_temperature: fill(-6),
            precipitation_probability: fill(90),
            precipitation: fill(0.4),
            snowfall: fill(0),
            wind_speed_10m: fill(12),
            wind_gusts_10m: fill(25),
            uv_index: fill(1),
            weather_code: fill(61),
        },
        daily: {
            time: days,
            weather_code: days.map(() => 61),
            temperature_2m_max: days.map(() => 1),
            temperature_2m_min: days.map(() => -4),
            precipitation_probability_max: days.map(() => 90),
            precipitation_sum: days.map(() => 5),
            sunrise: days.map((d) => `${d}T07:00`),
            sunset: days.map((d) => `${d}T17:30`),
            uv_index_max: days.map(() => 1),
        },
    };
}

createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    try {
        if (url.pathname === "/v1/forecast") {
            res.writeHead(200, { "content-type": "application/json" });
            res.end(
                JSON.stringify(
                    forecast(url.searchParams.get("timezone") ?? "UTC"),
                ),
            );
            return;
        }
        const m = /^\/ics\/([a-z0-9-]+\.ics)$/.exec(url.pathname);
        if (m) {
            const text = (await readFile(join(dir, m[1]), "utf8")).replace(
                /\{\{d([+-]\d+)\}\}/g,
                (_, n) => ymd(Number(n)),
            );
            res.writeHead(200, {
                "content-type": "text/calendar; charset=utf-8",
            });
            res.end(text.replace(/\r?\n/g, "\r\n"));
            return;
        }
        res.writeHead(404).end("not found");
    } catch {
        res.writeHead(404).end("not found");
    }
}).listen(port, "127.0.0.1", () => console.log(`fixture server on :${port}`));
