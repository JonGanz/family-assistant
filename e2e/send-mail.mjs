// CLI: deliver a fictional mail to a (GreenMail) SMTP server, e.g. the dev one from deploy/compose.dev.yaml.
//   node e2e/send-mail.mjs --port 3025 --to family@example.com --fixture promotion
//   node e2e/send-mail.mjs --port 3025 --to family@example.com --subject "Hi" --body "A test"
// Fixtures are e2e/fixtures/mail/<name>.eml (see mail-send.ts). Node 22.18+ (runs the .ts helper), no dependencies.
import { sendFixture, sendRaw } from "./mail-send.ts";

{
    const a = Object.fromEntries(
        process.argv.slice(2).reduce((acc, v, i, all) => (v.startsWith("--") ? [...acc, [v.slice(2), all[i + 1]]] : acc), []),
    );
    if (!a.port || !a.to) {
        console.error("usage: send-mail.mjs --port N --to ADDR (--fixture NAME | --subject S --body B)");
        process.exit(2);
    }
    if (a.fixture) await sendFixture(a.fixture, { port: a.port, to: a.to });
    else
        await sendRaw({
            port: a.port,
            to: a.to,
            raw: `From: Test Sender <sender@example.net>\nTo: ${a.to}\nSubject: ${a.subject ?? "Test"}\nDate: ${new Date().toUTCString()}\nMessage-ID: <${Date.now()}@dev.example.com>\nContent-Type: text/plain; charset=utf-8\n\n${a.body ?? "A test message."}\n`,
        });
    console.log("sent");
}
