// Fixture server for the e2e stack (no dependencies, Node 18+).
//   GET /ics/<name>.ics   files from e2e/fixtures/ics with {{d+N}} replaced by the household-local date
//                         (YYYYMMDD) N days from today, so the fixtures never go stale
//   POST /control/mild?latitude=<lat>&on=1|0   forecasts for that latitude become mild (warm, dry,
//                         calm) or go back to the freezing default, so a spec can make a rule stop matching
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
// The fixtures and the e2e household use America/New_York; a UTC date is a day ahead in the
// evening there, which pushed events out of the dashboard window.
const FIXTURE_TZ = "America/New_York";
function ymd(offsetDays) {
    // Calendar-day arithmetic on the New York date: n * 24 h is a day off across a DST change.
    const [y, m, d] = new Intl.DateTimeFormat("en-CA", { timeZone: FIXTURE_TZ })
        .format(new Date())
        .split("-")
        .map(Number);
    return new Date(Date.UTC(y, m - 1, d + offsetDays))
        .toISOString()
        .slice(0, 10)
        .replaceAll("-", "");
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

const mildLatitudes = new Set();

function forecast(tz, mild = false) {
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
            temperature_2m: mild ? 18 : -3,
            apparent_temperature: mild ? 18 : -6,
            weather_code: mild ? 0 : 61,
            wind_speed_10m: mild ? 5 : 12,
            wind_gusts_10m: mild ? 8 : 25,
            precipitation: mild ? 0 : 0.4,
            is_day: 1,
        },
        hourly: {
            time,
            temperature_2m: fill(mild ? 18 : -3),
            apparent_temperature: fill(mild ? 18 : -6),
            precipitation_probability: fill(mild ? 0 : 90),
            precipitation: fill(mild ? 0 : 0.4),
            snowfall: fill(0),
            wind_speed_10m: fill(mild ? 5 : 12),
            wind_gusts_10m: fill(mild ? 8 : 25),
            uv_index: fill(1),
            weather_code: fill(mild ? 0 : 61),
        },
        daily: {
            time: days,
            weather_code: days.map(() => (mild ? 0 : 61)),
            temperature_2m_max: days.map(() => (mild ? 20 : 1)),
            temperature_2m_min: days.map(() => (mild ? 12 : -4)),
            precipitation_probability_max: days.map(() => (mild ? 0 : 90)),
            precipitation_sum: days.map(() => (mild ? 0 : 5)),
            sunrise: days.map((d) => `${d}T07:00`),
            sunset: days.map((d) => `${d}T17:30`),
            uv_index_max: days.map(() => 1),
        },
    };
}

createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    try {
        if (url.pathname === "/control/mild" && req.method === "POST") {
            const lat = Number(url.searchParams.get("latitude"));
            if (url.searchParams.get("on") === "0") mildLatitudes.delete(lat);
            else mildLatitudes.add(lat);
            res.writeHead(204).end();
            return;
        }
        if (url.pathname === "/v1/forecast") {
            const lat = Number(url.searchParams.get("latitude"));
            res.writeHead(200, { "content-type": "application/json" });
            res.end(
                JSON.stringify(
                    forecast(
                        url.searchParams.get("timezone") ?? "UTC",
                        mildLatitudes.has(lat),
                    ),
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
