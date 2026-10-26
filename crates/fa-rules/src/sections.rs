//! Newsletter sections and per-member relevance (Ingest.md 6.3).

use std::sync::LazyLock;

use fa_domain::{Gender, Relevance};
use regex::Regex;
use scraper::{ElementRef, Html, Node};

use crate::signals::{
    gender_hit, gender_terms, grade_mentions, grade_or_class_hit, member_grades, name_hit, tag_hit,
};
use crate::text::{collapse_ws, contains_any_word};
use crate::types::{Context, MemberCtx, MessageFacts};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: Option<String>,
    pub text: String,
}

const BLOCKS: [&str; 22] = [
    "p",
    "div",
    "li",
    "ul",
    "ol",
    "table",
    "tr",
    "td",
    "th",
    "tbody",
    "thead",
    "blockquote",
    "pre",
    "section",
    "article",
    "header",
    "footer",
    "main",
    "center",
    "h5",
    "h6",
    "dl",
];
const SKIP: [&str; 4] = ["script", "style", "head", "title"];

#[derive(Default)]
struct Builder {
    out: Vec<Section>,
    title: Option<String>,
    paras: Vec<String>,
    inline: String,
}

impl Builder {
    fn end_para(&mut self) {
        let p = collapse_ws(&self.inline);
        self.inline.clear();
        if !p.is_empty() {
            self.paras.push(p);
        }
    }

    fn flush(&mut self) {
        self.end_para();
        if !self.paras.is_empty() {
            self.out.push(Section {
                title: self.title.take(),
                text: self.paras.join("\n\n"),
            });
        } else if self.title.is_some() {
            // A heading with nothing under it is dropped.
            self.title = None;
        }
        self.paras.clear();
    }

    fn start(&mut self, title: String) {
        self.flush();
        let t = collapse_ws(&title);
        self.title = (!t.is_empty()).then_some(t);
    }
}

fn el_text(el: ElementRef<'_>) -> String {
    collapse_ws(&el.text().collect::<Vec<_>>().join(" "))
}

/// True when every non-blank text node under `el` sits inside `b` or `strong`.
fn is_bold_only(el: ElementRef<'_>) -> bool {
    fn has_plain_text(el: ElementRef<'_>, in_bold: bool) -> bool {
        let bold = in_bold || matches!(el.value().name(), "b" | "strong");
        el.children().any(|c| match c.value() {
            Node::Text(t) => !bold && !t.trim().is_empty(),
            Node::Element(_) => ElementRef::wrap(c).is_some_and(|ce| has_plain_text(ce, bold)),
            _ => false,
        })
    }
    !el_text(el).is_empty() && !has_plain_text(el, false)
}

fn walk(el: ElementRef<'_>, b: &mut Builder) {
    let name = el.value().name();
    if SKIP.contains(&name) {
        return;
    }
    if matches!(name, "h1" | "h2" | "h3" | "h4") {
        b.end_para();
        b.start(el_text(el));
        return;
    }
    if name == "hr" {
        b.end_para();
        b.flush();
        return;
    }
    if name == "br" {
        b.inline.push(' ');
        return;
    }
    if matches!(name, "p" | "div") {
        let t = el_text(el);
        if t.chars().count() <= 100 && is_bold_only(el) {
            b.end_para();
            b.start(t);
            return;
        }
    }
    let block = BLOCKS.contains(&name);
    if block {
        b.end_para();
    }
    for c in el.children() {
        match c.value() {
            Node::Text(t) => b.inline.push_str(t),
            Node::Element(_) => {
                if let Some(ce) = ElementRef::wrap(c) {
                    walk(ce, b);
                }
            }
            _ => {}
        }
    }
    if block {
        b.end_para();
    }
}

fn html_sections(html: &str) -> Vec<Section> {
    let doc = Html::parse_document(html);
    let mut b = Builder::default();
    walk(doc.root_element(), &mut b);
    b.flush();
    b.out
}

static SMALL_WORDS: [&str; 12] = [
    "a", "an", "the", "of", "and", "for", "to", "in", "on", "at", "&", "or",
];

fn is_title_line(line: &str) -> bool {
    let l = line.trim();
    if l.is_empty() || l.chars().count() > 60 {
        return false;
    }
    if l.ends_with(':') {
        return true;
    }
    if l.ends_with(['.', '?', '!', ',', ';']) {
        return false;
    }
    let words: Vec<&str> = l.split_whitespace().collect();
    if words.is_empty() {
        return false;
    }
    words.iter().enumerate().all(|(i, w)| {
        let first = w
            .trim_start_matches(|c: char| !c.is_alphanumeric())
            .chars()
            .next();
        match first {
            Some(c) if c.is_alphabetic() => {
                c.is_uppercase() || (i > 0 && SMALL_WORDS.contains(&w.to_lowercase().as_str()))
            }
            _ => true,
        }
    }) && words.iter().any(|w| w.chars().any(char::is_alphabetic))
}

static BLANK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\r?\n[ \t]*\r?\n").expect("static"));

fn text_sections(text: &str) -> Vec<Section> {
    let mut b = Builder::default();
    for block in BLANK.split(text) {
        let block = block.trim_matches(['\r', '\n']);
        if block.trim().is_empty() {
            continue;
        }
        let mut lines = block.lines();
        let first = lines.next().unwrap_or_default();
        if is_title_line(first) {
            b.start(first.trim().trim_end_matches(':').to_owned());
            let rest: Vec<&str> = lines.collect();
            let rest = collapse_ws(&rest.join(" "));
            if !rest.is_empty() {
                b.paras.push(rest);
            }
        } else {
            b.paras.push(collapse_ws(block));
        }
    }
    b.flush();
    b.out
}

/// HTML: split on `h1`-`h4`, `hr` and bold-only paragraphs. Plain text: blank-line blocks
/// whose first line is short and Title Case or ends with ':'. Always at least one section
/// when there is any text.
pub fn sections(f: &MessageFacts<'_>) -> Vec<Section> {
    let mut found = f.body_html.map(html_sections).unwrap_or_default();
    if found.is_empty() {
        found = text_sections(f.body_text);
    }
    if found.is_empty() && !f.body_text.trim().is_empty() {
        found.push(Section {
            title: None,
            text: collapse_ws(f.body_text),
        });
    }
    found
}

/// Relevant when it names the member, their grade or class, a tag, or a gender term for
/// their gender; Other when it names only other grades or only the other gender's terms;
/// otherwise General.
pub fn relevance(section: &Section, member: &MemberCtx, ctx: &Context) -> Relevance {
    let text = match &section.title {
        Some(t) => format!("{t}\n{}", section.text),
        None => section.text.clone(),
    };
    let mentions = grade_mentions(&text);
    if name_hit(member, &text)
        || tag_hit(member, &text)
        || gender_hit(member, ctx, &text)
        || grade_or_class_hit(member, None, &text, &mentions)
    {
        return Relevance::Relevant;
    }
    let other_grade = !mentions.is_empty() && !member_grades(member, None).is_empty();
    let other_gender = match member.gender {
        Some(g @ (Gender::Female | Gender::Male)) => {
            let other = if g == Gender::Female {
                Gender::Male
            } else {
                Gender::Female
            };
            contains_any_word(&text, gender_terms(ctx, other))
        }
        _ => false,
    };
    if other_grade || other_gender {
        Relevance::Other
    } else {
        Relevance::General
    }
}
