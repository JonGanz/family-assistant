import sys; sys.path.insert(0, "tools")
from drawio import Page, write, NODE, COMP, EXT, ART, TITLE, NOTE, FRAME

# Page 1: Components (UML 2 component diagram)
pc = Page("components", "Components", 1400, 760)
b, e = pc.box, pc.edge
b("AI: LLM extraction slice (UML 2 component), DRAFT v1 (M4, D-093, D-094)", 20, 10, 1000, 30, TITLE)
api = b("fa-api\n/ai/llm-endpoint · /ai/settings · /ai/pipeline · /ai/stats · /ai/recipes/extract", 380, 60, 460, 60, COMP)
srv = b("fa-server\nwiring · worker · eval subcommand", 940, 60, 260, 60, COMP)
ml = b("fa-mail\nmail.process (enqueue) · mail.llm job · llm.prune", 60, 220, 320, 70, COMP)
pl = b("fa-pipeline\npending_llm · record_stage · set_llm_stage", 440, 220, 320, 70, COMP)
llm = b("fa-llm\nLlmClient (openai_chat | ollama_native) · Provider (cache, Semaphore(1), llm_call)", 820, 220, 420, 70, COMP)
rec = b("fa-recipe\nfrom_html (JSON-LD) · from_llm · fetch_page · RecipeDraft", 820, 380, 420, 60, COMP)
ev = b("fa-eval\nreplay in_eval_sample: rules vs rules+llm", 440, 380, 320, 60, COMP)
con = b("fa-connectors\nHttp (private hosts blocked)", 820, 520, 300, 50, COMP)
st = b("fa-store\nllm_endpoint · ai_settings · llm_call · extracted_by · decision", 60, 520, 460, 60, COMP)
srvx = b("«external» OpenAI-compatible LLM server\nPOST /v1/chat/completions (json_schema) · GET /v1/models\ndefault: Ollama on the GPU node", 820, 640, 460, 70, EXT)
web = b("«external» recipe web page", 1180, 520, 180, 50, EXT)
for s, t, l in [(api, llm, "test · models · invalidate"), (api, rec, "extract"), (api, pl, "set_llm_stage"), (api, st, "endpoint, settings, stats"),
                (srv, ml, "handlers"), (srv, ev, "eval"), (ml, pl, "run · record_stage"), (ml, llm, "Provider::call(mail.llm)"),
                (ml, st, "jobs, proposals"), (ev, pl, "replay"), (ev, llm, "purpose=eval"), (rec, llm, "fallback"), (rec, con, "fetch_page"),
                (llm, st, "endpoint + key, llm_call")]:
    e(s, t, l)
e(llm, srvx, "HTTP(S), Bearer key optional, redirects off", False)
e(con, web, "HTTPS GET", False)
b("No prompt or response text is stored or logged. Extracted items still wait for approval (D-057, D-060). M4.1: fa-decide (System One), kNN, fa-backends, fa-gpu-agent", 60, 640, 460, 70, NOTE)

# Page 2: Deployment (UML 2 deployment)
pd = Page("deploy", "Deployment", 1300, 620)
b = pd.box
b("AI: Deployment (UML 2), reference hardware, DRAFT v1 (M4)", 20, 10, 800, 30, TITLE)
core = b("«device» Core node (always on)\n«executionEnvironment» Docker Compose", 40, 60, 460, 300, NODE)
app = b("«artifact» app: fa-server serve all\n(api + worker: mail.llm, llm.prune)", 70, 130, 400, 60, ART)
pg = b("«artifact» postgres:16 + pgvector\nllm_endpoint · ai_settings · llm_call", 70, 220, 400, 60, ART)
gpu = b("«device» GPU node 0..*: ref. Linux PC, RTX 3070 8 GB (may be asleep)", 640, 60, 560, 300, NODE)
ol = b("«artifact» Ollama (hoster-installed)\nOLLAMA_HOST=LAN-or-tailnet-IP:11434\nmodel qwen3:8b (Q4)", 680, 130, 300, 80, ART)
ag = b("«artifact» fa-gpu-agent (M4.1, optional)", 680, 240, 300, 50, ART)
pd.edge(app, ol, "OpenAI-compatible HTTP over LAN / tailnet", False)
pd.edge(app, pg, "sqlx (RLS)")
b("Any OpenAI-compatible server works (llama-server, vLLM, LM Studio): base_url + model + optional key in Settings → AI. Single-machine installs point base_url at 127.0.0.1", 40, 420, 600, 70, NOTE)

# Page 3: Sequence (UML 2): a message whose rules fall below threshold
ps = Page("sequence", "LLM sequence", 1500, 760)
ps.box("AI: a message the rules can't settle (UML 2 sequence), DRAFT v1", 20, 10, 900, 30, TITLE)
ps.box("sd mail.llm", 20, 50, 1440, 670, FRAME)
lanes = ["fa-mail (mail.process)", "fa-pipeline", "fa-store / Postgres", "fa-mail (mail.llm job)", "fa_llm::Provider", "LLM server"]
xs = {}
for i, n in enumerate(lanes):
    x = 50 + i * 230
    ps.box(n, x, 90, 200, 36, "html=1;whiteSpace=wrap;")
    ps.box("", x + 99, 126, 2, 570, "html=1;dashed=1;strokeColor=#999999;")
    xs[n] = x + 95
def msg(a, c, y, label, dashed=False):
    s = ps.box("", xs[a], y, 10, 10, "html=1;fillColor=#dae8fc;")
    t = ps.box("", xs[c], y, 10, 10, "html=1;fillColor=#dae8fc;")
    ps.edge(s, t, label, dashed)
msg("fa-mail (mail.process)", "fa-pipeline", 160, "run(email.category): rules 0.55 below 0.8 → next stage llm → Outcome{pending_llm}")
msg("fa-mail (mail.process)", "fa-store / Postgres", 210, "default value applies · enqueue mail.llm{message_id} (key mail.llm:id, max_attempts 12)")
msg("fa-mail (mail.llm job)", "fa_llm::Provider", 270, "call::MailLlmResult(household, mail.llm, prompt + schema)")
msg("fa_llm::Provider", "fa-store / Postgres", 310, "client_for: llm_endpoint + api_key secret (30 s cache) · Semaphore(1)")
msg("fa_llm::Provider", "LLM server", 360, "POST /chat/completions response_format json_schema, temperature 0")
msg("LLM server", "fa_llm::Provider", 410, "choices[0].message.content (JSON) · usage", True)
msg("fa_llm::Provider", "fa-store / Postgres", 450, "llm_call{purpose, model, latency_ms, ok, tokens} (no content)")
msg("fa-mail (mail.llm job)", "fa-pipeline", 500, "record_stage(Llm, candidates) ≥ 0.7 → Decision(stage_kind=llm), else → human")
msg("fa-mail (mail.llm job)", "fa-store / Postgres", 550, "category changed → re-run behaviour · events/bills/tasks (extracted_by=llm) → existing proposal paths → inbox")
msg("fa-mail (mail.llm job)", "fa-store / Postgres", 610, "Unavailable / Timeout → job fails → retry 30 s × 2^n (≈1 day), defaults stay", True)

write("arch/AI.drawio", [pc, pd, ps])
