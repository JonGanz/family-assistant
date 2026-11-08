import { readFileSync } from "node:fs";
import {
  expect,
  test,
  type APIRequestContext,
  type Playwright,
} from "@playwright/test";

// M5 API specs: recipes (import, save, image), the ingredient catalogue, the meal plan, templates,
// the grocery list and the dashboard meals. They need the stack from scripts/e2e.sh: the recipe
// pages come from the fixture server (e2e/fixtures/recipes), and there is no LLM, so a page without
// structured data is refused with 503. Everything is fictional (D-035); the specs delete what they
// make, because the UI specs run on the same stack afterwards.
const BASE = process.env.FA_E2E_BASE_URL ?? "http://127.0.0.1:18080";
const FIX = process.env.FA_E2E_FIXTURES ?? "http://127.0.0.1:18091";
const seed = JSON.parse(
  readFileSync(process.env.FA_E2E_SEED ?? "", "utf8"),
) as { device: { token: string } };
const NY = "America/New_York";

/* eslint-disable @typescript-eslint/no-explicit-any */
type J = any;

/** The New York calendar date N days from today (the e2e household's time zone). */
const day = (n: number) => {
  const [y, m, d] = new Intl.DateTimeFormat("en-CA", { timeZone: NY })
    .format(new Date())
    .split("-")
    .map(Number);
  return new Date(Date.UTC(y, m - 1, d + n)).toISOString().slice(0, 10);
};

async function login(pw: Playwright, user: string, password: string) {
  const ctx = await pw.request.newContext({
    baseURL: BASE,
    extraHTTPHeaders: { "X-FA-Request": "1" },
  });
  const res = await ctx.post("/api/v1/auth/login", {
    data: { username: user, password },
  });
  expect(res.ok()).toBeTruthy();
  return ctx;
}

let alex: APIRequestContext;
let riley: APIRequestContext;
let device: APIRequestContext;

test.beforeAll(async ({ playwright }) => {
  alex = await login(
    playwright,
    process.env.FA_E2E_USER ?? "alex@example.com",
    process.env.FA_E2E_PASSWORD ?? "",
  );
  riley = await login(
    playwright,
    process.env.FA_E2E_CHILD_USER ?? "riley@example.com",
    process.env.FA_E2E_CHILD_PASSWORD ?? "",
  );
  device = await playwright.request.newContext({
    baseURL: BASE,
    extraHTTPHeaders: {
      "X-FA-Request": "1",
      Authorization: `Bearer ${seed.device.token}`,
    },
  });
});

test.describe.configure({ mode: "serial" });

const ok = async (res: {
  ok(): boolean;
  status(): number;
  text(): Promise<string>;
}) => {
  if (!res.ok()) throw new Error(`HTTP ${res.status()}: ${await res.text()}`);
  return res;
};
const json = async (res: any): Promise<J> => (await ok(res)).json();

// State shared by the serial tests, and what to delete at the end.
let slots: J[] = [];
let dinner: J;
let lunch: J;
let lentil: J; // saved from the JSON-LD page
let soup: J; // saved with a staple line
let stapleName = "";
const entries: string[] = [];
const templates: string[] = [];
const groceryItems: string[] = [];
let stapleIngredient = "";

const importUrl = (name: string) => ({
  url: `${FIX}/recipes/${name}`,
});

test("recipe import: JSON-LD and microdata pages give a draft with lines", async () => {
  const ld = await json(
    await alex.post("/api/v1/recipes/import", {
      data: importUrl("jsonld.html"),
    }),
  );
  expect(ld.origin).toBe("jsonld");
  expect(ld.name).toBe("Fictional Lentil Stew");
  expect(ld.ingredients.length).toBe(6);
  expect(ld.steps.length).toBe(2);
  expect(ld.image_url).toContain("/recipes/image.png");

  const md = await json(
    await alex.post("/api/v1/recipes/import", {
      data: importUrl("microdata.html"),
    }),
  );
  expect(md.origin).toBe("microdata");
  expect(md.name).toBe("Fictional Pumpkin Pasta");
  expect(md.ingredients.length).toBe(4);

  // The old path stays as an alias for one milestone.
  const alias = await alex.post("/api/v1/ai/recipes/extract", {
    data: importUrl("jsonld.html"),
  });
  expect(alias.ok()).toBeTruthy();
});

test("recipe import: no structured data and no LLM is 503 llm_unavailable; a child cannot import", async () => {
  const res = await alex.post("/api/v1/recipes/import", {
    data: importUrl("plain.html"),
  });
  expect(res.status()).toBe(503);
  expect((await res.json()).code).toBe("llm_unavailable");
  expect(
    (
      await riley.post("/api/v1/recipes/import", {
        data: importUrl("jsonld.html"),
      })
    ).status(),
  ).toBe(403);
});

test("save a recipe from the draft; the image is fetched and stored (200, then 304)", async () => {
  const draft = await json(
    await alex.post("/api/v1/recipes/import", {
      data: importUrl("jsonld.html"),
    }),
  );
  const res = await alex.post("/api/v1/recipes", {
    data: {
      title: draft.name,
      source_url: draft.source_url ?? importUrl("jsonld.html").url,
      servings: draft.servings ?? 4,
      yield_text: draft.yield_text,
      prep_minutes: draft.prep_minutes,
      cook_minutes: draft.cook_minutes,
      total_minutes: draft.total_minutes,
      import_method: draft.origin,
      tags: ["e2e", "soup"],
      image_url: draft.image_url,
      lines: draft.ingredients.map((i: J) => i.text),
      steps: draft.steps,
    },
  });
  expect(res.status()).toBe(201);
  lentil = await res.json();
  expect(lentil.lines.length).toBe(6);
  // The seed catalogue matches at least the lentils and the carrots.
  const matched = lentil.lines.filter((l: J) => l.match_state === "matched");
  expect(matched.length).toBeGreaterThanOrEqual(2);

  // The worker stores the image (job recipe.image).
  let img: Awaited<ReturnType<typeof alex.get>> | undefined;
  await expect
    .poll(
      async () => {
        img = await alex.get(`/api/v1/recipes/${lentil.id}/image`);
        return img.status();
      },
      { timeout: 30_000 },
    )
    .toBe(200);
  expect(img!.headers()["content-type"]).toMatch(/^image\//);
  const tag = img!.headers()["etag"];
  expect(tag).toBeTruthy();
  const again = await alex.get(`/api/v1/recipes/${lentil.id}/image`, {
    headers: { "If-None-Match": tag },
  });
  expect(again.status()).toBe(304);

  const view = await json(await alex.get(`/api/v1/recipes/${lentil.id}`));
  expect(view.has_image).toBe(true);
  const list = await json(await alex.get("/api/v1/recipes?q=lentil"));
  expect(list.items.map((r: J) => r.id)).toContain(lentil.id);
});

test("the seed catalogue exists, and the add-box suggestions filter it", async () => {
  const all = await json(await alex.get("/api/v1/ingredients"));
  expect(all.length).toBeGreaterThan(200);
  const some = await json(await alex.get("/api/v1/ingredients?q=carrot"));
  expect(some.length).toBeGreaterThan(0);
  expect(some.length).toBeLessThan(all.length);
});

test("ratings: any member rates for themself; an adult favourites", async () => {
  const r = await json(
    await riley.put(`/api/v1/recipes/${lentil.id}/rating`, {
      data: { stars: 4 },
    }),
  );
  expect(r.mine).toBe(4);
  const fav = await alex.put(`/api/v1/recipes/${lentil.id}/favourite`, {
    data: { favourite: true },
  });
  expect(fav.status()).toBe(204);
});

test("plan: several entries in one slot, then a swap", async () => {
  slots = await json(await alex.get("/api/v1/meals/slots"));
  dinner = slots.find((s) => s.name === "Dinner");
  lunch = slots.find((s) => s.name === "Lunch");
  expect(dinner && lunch).toBeTruthy();

  const d = day(3);
  const a = await json(
    await alex.post("/api/v1/meal-plan", {
      data: { date: d, slot_id: dinner.id, recipe_id: lentil.id, servings: 4 },
    }),
  );
  const b = await json(
    await alex.post("/api/v1/meal-plan", {
      data: { date: d, slot_id: dinner.id, free_text: "Fictional side salad" },
    }),
  );
  const first = a.entries.find((e: J) => e.recipe_id === lentil.id);
  const second = b.entries.find(
    (e: J) => e.free_text === "Fictional side salad",
  );
  entries.push(first.id, second.id);
  expect(second.slot_id).toBe(dinner.id);
  expect(second.ord).toBeGreaterThan(first.ord);
  expect(first.title).toBe("Fictional Lentil Stew");

  // Swap with an entry in another slot of another day.
  const other = await json(
    await alex.post("/api/v1/meal-plan", {
      data: {
        date: day(4),
        slot_id: lunch.id,
        free_text: "Fictional sandwiches",
      },
    }),
  );
  const third = other.entries.find(
    (e: J) => e.free_text === "Fictional sandwiches",
  );
  entries.push(third.id);
  const swapped = await json(
    await alex.post("/api/v1/meal-plan/swap", {
      data: { a: first.id, b: third.id },
    }),
  );
  expect(swapped.change_id).toBeTruthy();
  const range = await json(
    await alex.get(`/api/v1/meal-plan?from=${day(3)}&to=${day(4)}`),
  );
  const at = (id: string) => range.find((e: J) => e.id === id);
  expect(at(first.id).date).toBe(day(4));
  expect(at(first.id).slot_id).toBe(lunch.id);
  expect(at(third.id).date).toBe(day(3));
  expect(at(third.id).slot_id).toBe(dinner.id);
});

test("templates: save from a plan range, duplicate, preview, apply with one Add slot, undo", async () => {
  const from = await alex.post("/api/v1/meal-templates/from-plan", {
    data: {
      name: "E2E fictional week",
      from: day(3),
      to: day(4),
      slot_ids: [dinner.id, lunch.id],
    },
  });
  expect(from.status()).toBe(201);
  const tpl = await from.json();
  templates.push(tpl.id);
  expect(tpl.days).toBe(2);
  expect(tpl.cells.length).toBe(3);

  const dupRes = await alex.post(`/api/v1/meal-templates/${tpl.id}/duplicate`, {
    data: { name: "E2E fictional copy" },
  });
  expect(dupRes.status()).toBe(201);
  const dup = await dupRes.json();
  templates.push(dup.id);
  expect(dup.cells.length).toBe(tpl.cells.length);
  expect(dup.id).not.toBe(tpl.id);

  // Apply onto two empty days further on, with a free-text entry in one dinner slot.
  const start = day(10);
  const pre = await json(
    await alex.post(`/api/v1/meal-templates/${tpl.id}/preview`, {
      data: { start },
    }),
  );
  expect(pre.targets.length).toBeGreaterThanOrEqual(2);
  const mine = await json(
    await alex.post("/api/v1/meal-plan", {
      data: {
        date: start,
        slot_id: dinner.id,
        free_text: "Fictional leftovers",
      },
    }),
  );
  const existing = mine.entries.find(
    (e: J) => e.free_text === "Fictional leftovers",
  );
  entries.push(existing.id);

  const applied = await json(
    await alex.post(`/api/v1/meal-templates/${tpl.id}/apply`, {
      data: { start, add_slots: [{ date: start, slot_id: dinner.id }] },
    }),
  );
  expect(applied.change_id).toBeTruthy();
  const after = await json(
    await alex.get(`/api/v1/meal-plan?from=${start}&to=${day(11)}`),
  );
  for (const e of after) entries.push(e.id);
  // Add: the leftovers are still there next to the template's item.
  const dinnerDay1 = after.filter(
    (e: J) => e.date === start && e.slot_id === dinner.id,
  );
  expect(dinnerDay1.map((e: J) => e.title)).toContain("Fictional leftovers");
  expect(dinnerDay1.length).toBe(3); // the leftovers plus the two template items

  await ok(
    await alex.post(`/api/v1/meals/changes/${applied.change_id}/undo`, {
      data: {},
    }),
  );
  const undone = await json(
    await alex.get(`/api/v1/meal-plan?from=${start}&to=${day(11)}`),
  );
  expect(undone.map((e: J) => e.title)).toEqual(["Fictional leftovers"]);
  const twice = await alex.post(
    `/api/v1/meals/changes/${applied.change_id}/undo`,
    {
      data: {},
    },
  );
  expect(twice.status()).toBe(409);
});

test("grocery: generate twice is idempotent, staples get their own section, reconcile after a delete", async () => {
  // A staple of our own, so the seed catalogue stays as it is.
  stapleName = `fictional pepper blend ${Date.now()}`;
  const ing = await json(
    await alex.post("/api/v1/ingredients", {
      data: { canonical_name: stapleName, aliases: [], is_staple: true },
    }),
  );
  stapleIngredient = ing.id;
  const res = await alex.post("/api/v1/recipes", {
    data: {
      title: "E2E staple soup",
      import_method: "manual",
      servings: 2,
      tags: [],
      lines: ["2 carrots", `1 tsp ${stapleName}`],
      steps: ["Stir."],
    },
  });
  expect(res.status()).toBe(201);
  soup = await res.json();

  const planned = await json(
    await alex.post("/api/v1/meal-plan", {
      data: {
        date: day(6),
        slot_id: dinner.id,
        recipe_id: soup.id,
        servings: 2,
      },
    }),
  );
  const entry = planned.entries.find((e: J) => e.recipe_id === soup.id);
  entries.push(entry.id);

  const gen = await json(
    await alex.post("/api/v1/grocery/generate", {
      data: { entry_ids: [entry.id] },
    }),
  );
  expect(gen.added + gen.merged).toBeGreaterThanOrEqual(1);
  expect(gen.staples).toBe(1);
  const again = await json(
    await alex.post("/api/v1/grocery/generate", {
      data: { entry_ids: [entry.id] },
    }),
  );
  expect(again.added).toBe(0);
  expect(again.staples).toBe(0);

  let g = await json(await alex.get("/api/v1/grocery"));
  expect(g.staples.map((s: J) => s.name)).toContain(stapleName);
  const carrots = g.items.filter((i: J) =>
    i.sources.some((s: J) => s.entry_id === entry.id),
  );
  expect(carrots.length).toBeGreaterThanOrEqual(1);
  const before = carrots[0];
  for (const i of g.items) groceryItems.push(i.id);

  // Deleting the entry reports an impact; reconcile removes the contribution.
  const del = await json(await alex.delete(`/api/v1/meal-plan/${entry.id}`));
  expect(del.grocery_impact.length).toBeGreaterThanOrEqual(1);
  expect(del.grocery_impact.some((x: J) => x.item_id === before.id)).toBe(true);
  const rec = await json(
    await alex.post("/api/v1/grocery/reconcile", {
      data: { entry_ids: [entry.id] },
    }),
  );
  expect(rec.changed).toBeGreaterThanOrEqual(1);
  g = await json(await alex.get("/api/v1/grocery"));
  expect(
    g.items.filter((i: J) => i.sources.some((s: J) => s.entry_id === entry.id)),
  ).toHaveLength(0);
  // Nothing else needed the carrots, so the item itself is gone (impact `remove`).
  expect(g.items.some((i: J) => i.id === before.id)).toBe(false);
});

test("stores: a store's own aisle order and hidden aisles round-trip and items keep the store", async () => {
  const stores = await json(
    await alex.put("/api/v1/meals/stores", {
      data: [{ name: "Fictional Market", ord: 0 }],
    }),
  );
  const store = stores.find((s: J) => s.name === "Fictional Market");
  const aisles = await json(
    await alex.get(`/api/v1/meals/stores/${store.id}/aisles`),
  );
  expect(aisles.length).toBeGreaterThan(2);
  // Reverse the order and hide the first one.
  const reversed = [...aisles]
    .reverse()
    .map((a: J, i: number) => ({
      aisle_id: a.aisle_id,
      ord: i,
      hidden: i === 0,
    }));
  await ok(
    await alex.put(`/api/v1/meals/stores/${store.id}/aisles`, {
      data: reversed,
    }),
  );
  const back = await json(
    await alex.get(`/api/v1/meals/stores/${store.id}/aisles`),
  );
  expect(back.map((a: J) => a.aisle_id)).toEqual(
    reversed.map((a: J) => a.aisle_id),
  );
  expect(back[0].hidden).toBe(true);

  const item = await json(
    await alex.post("/api/v1/grocery/items", {
      data: { text: "fictional tea", store_id: store.id },
    }),
  );
  groceryItems.push(item.id);
  const g = await json(await alex.get("/api/v1/grocery"));
  expect(g.stores.map((s: J) => s.name)).toContain("Fictional Market");
  expect(g.items.find((i: J) => i.id === item.id).store_id).toBe(store.id);

  // Removing the store nulls its items.
  await ok(await alex.put("/api/v1/meals/stores", { data: [] }));
  const g2 = await json(await alex.get("/api/v1/grocery"));
  expect(g2.items.find((i: J) => i.id === item.id).store_id ?? null).toBeNull();
});

test("authz: a child adds and checks, gets 403 on delete; the device adds without a PIN", async () => {
  const id = crypto.randomUUID();
  const add = await riley.post("/api/v1/grocery/items", {
    data: { id, text: "fictional oat milk" },
  });
  expect(add.status()).toBe(201);
  groceryItems.push(id);
  // The same id again is a replay: 200, not a second item (D-118).
  const replay = await riley.post("/api/v1/grocery/items", {
    data: { id, text: "fictional oat milk" },
  });
  expect(replay.status()).toBe(200);

  const checked = await json(
    await riley.post(`/api/v1/grocery/items/${id}/check`, { data: {} }),
  );
  expect(checked.checked_at).toBeTruthy();
  const unchecked = await json(
    await riley.post(`/api/v1/grocery/items/${id}/uncheck`, { data: {} }),
  );
  expect(unchecked.checked_at ?? null).toBeNull();

  expect((await riley.delete(`/api/v1/grocery/items/${id}`)).status()).toBe(
    403,
  );
  const edit = await riley.put(`/api/v1/grocery/items/${id}`, {
    data: { text: "other" },
    headers: { "If-Match": `"${checked.version}"` },
  });
  expect(edit.status()).toBe(403);
  expect(
    (
      await riley.post("/api/v1/meal-plan", {
        data: { date: day(5), slot_id: dinner.id, free_text: "nope" },
      })
    ).status(),
  ).toBe(403);

  const devId = crypto.randomUUID();
  const dev = await device.post("/api/v1/grocery/items", {
    data: { id: devId, text: "fictional kiosk bread" },
  });
  expect(dev.status()).toBe(201);
  groceryItems.push(devId);
  expect(
    (
      await device.post(`/api/v1/grocery/items/${devId}/check`, { data: {} })
    ).ok(),
  ).toBeTruthy();
  // Without a PIN the device cannot delete.
  expect((await device.delete(`/api/v1/grocery/items/${devId}`)).status()).toBe(
    403,
  );
  // An adult can.
  expect((await alex.delete(`/api/v1/grocery/items/${devId}`)).status()).toBe(
    204,
  );
});

test("GET /dashboard carries today's meals", async () => {
  const made = await json(
    await alex.post("/api/v1/meal-plan", {
      data: {
        date: day(0),
        slot_id: dinner.id,
        free_text: "Fictional dashboard dinner",
      },
    }),
  );
  const e = made.entries.find(
    (x: J) => x.free_text === "Fictional dashboard dinner",
  );
  entries.push(e.id);
  const dash = await json(await device.get("/api/v1/dashboard"));
  expect(Array.isArray(dash.meals)).toBe(true);
  const m = dash.meals.find((x: J) => x.title === "Fictional dashboard dinner");
  expect(m).toBeTruthy();
  expect(m.slot_name).toBe("Dinner");
  expect(m.time).toMatch(/^\d\d:\d\d$/);
});

test.afterAll(async () => {
  // Leave the shared stack as found: the UI specs start from an empty plan and list.
  for (const id of entries)
    await alex.delete(`/api/v1/meal-plan/${id}`).catch(() => {});
  for (const id of templates)
    await alex.delete(`/api/v1/meal-templates/${id}`).catch(() => {});
  const g = await alex
    .get("/api/v1/grocery")
    .then((r) => r.json())
    .catch(() => null);
  if (g)
    for (const i of g.items)
      await alex.delete(`/api/v1/grocery/items/${i.id}`).catch(() => {});
  for (const id of [lentil?.id, soup?.id].filter(Boolean))
    await alex.delete(`/api/v1/recipes/${id}`).catch(() => {});
  if (stapleIngredient)
    await alex
      .delete(`/api/v1/ingredients/${stapleIngredient}`)
      .catch(() => {});
  await alex.put("/api/v1/meals/stores", { data: [] }).catch(() => {});
});
