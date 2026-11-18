//! Converts TipTap document JSON into publishable Markdown and HTML.
//!
//! Internal `[[wiki-links]]` and source mentions are written out as their plain
//! label: they point into the private case file, so they must not leak into
//! anything that is published.

use serde_json::Value;

fn children(node: &Value) -> &[Value] {
    node.get("content").and_then(Value::as_array).map_or(&[], |v| v.as_slice())
}

fn ty(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("")
}

fn attr_str<'a>(node: &'a Value, key: &str) -> Option<&'a str> {
    node.get("attrs")?.get(key)?.as_str()
}

fn has_mark(node: &Value, name: &str) -> bool {
    mark(node, name).is_some()
}

fn mark<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node.get("marks")?.as_array()?.iter().find(|m| ty(m) == name)
}

/// Only web and mail links survive; anything else (javascript:, file:, …) is dropped.
fn safe_url(u: &str) -> Option<&str> {
    let l = u.trim().to_ascii_lowercase();
    (l.starts_with("http://") || l.starts_with("https://") || l.starts_with("mailto:"))
        .then_some(u.trim())
}

pub fn parse(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or(Value::Null)
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn esc_md(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '>' | '~') {
            o.push('\\');
        }
        o.push(c);
    }
    o
}

// ---------- Markdown ----------

fn md_inline(nodes: &[Value]) -> String {
    let mut out = String::new();
    for n in nodes {
        match ty(n) {
            "text" => {
                let raw = n.get("text").and_then(Value::as_str).unwrap_or("");
                if raw.is_empty() {
                    continue;
                }
                let mut s = if has_mark(n, "code") {
                    format!("`{}`", raw.replace('`', "'"))
                } else {
                    esc_md(raw)
                };
                if has_mark(n, "bold") {
                    s = format!("**{s}**");
                }
                if has_mark(n, "italic") {
                    s = format!("*{s}*");
                }
                if has_mark(n, "strike") {
                    s = format!("~~{s}~~");
                }
                if let Some(url) = mark(n, "link").and_then(|m| attr_str(m, "href")).and_then(safe_url) {
                    s = format!("[{s}]({})", url.replace(')', "%29").replace(' ', "%20"));
                }
                out.push_str(&s);
            }
            "hardBreak" => out.push_str("  \n"),
            "mention" => out.push_str(&esc_md(attr_str(n, "label").unwrap_or(""))),
            _ => out.push_str(&md_inline(children(n))),
        }
    }
    out
}

fn indent_after_first(s: &str, pad: &str) -> String {
    s.lines()
        .enumerate()
        .map(|(i, l)| if i == 0 || l.is_empty() { l.to_string() } else { format!("{pad}{l}") })
        .collect::<Vec<_>>()
        .join("\n")
}

fn md_blocks(nodes: &[Value], sep: &str) -> String {
    nodes.iter().map(md_block).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(sep)
}

fn md_block(n: &Value) -> String {
    match ty(n) {
        "paragraph" => md_inline(children(n)),
        "heading" => {
            let lvl = n.get("attrs").and_then(|a| a.get("level")).and_then(Value::as_u64).unwrap_or(1);
            format!("{} {}", "#".repeat(lvl.clamp(1, 6) as usize), md_inline(children(n)))
        }
        "blockquote" => md_blocks(children(n), "\n\n")
            .lines()
            .map(|l| if l.is_empty() { ">".to_string() } else { format!("> {l}") })
            .collect::<Vec<_>>()
            .join("\n"),
        "codeBlock" => {
            let code: String = children(n)
                .iter()
                .filter_map(|c| c.get("text").and_then(Value::as_str))
                .collect();
            let mut fence = "```".to_string();
            while code.contains(&fence) {
                fence.push('`');
            }
            let lang = attr_str(n, "language").unwrap_or("");
            format!("{fence}{lang}\n{code}\n{fence}")
        }
        "horizontalRule" => "---".into(),
        "bulletList" | "orderedList" | "taskList" => {
            let ordered = ty(n) == "orderedList";
            let start = n.get("attrs").and_then(|a| a.get("start")).and_then(Value::as_u64).unwrap_or(1);
            children(n)
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let marker = if ty(n) == "taskList" {
                        let done = item.get("attrs").and_then(|a| a.get("checked")).and_then(Value::as_bool) == Some(true);
                        format!("- [{}] ", if done { "x" } else { " " })
                    } else if ordered {
                        format!("{}. ", start + i as u64)
                    } else {
                        "- ".to_string()
                    };
                    let body = md_blocks(children(item), "\n");
                    format!("{marker}{}", indent_after_first(&body, &" ".repeat(marker.len().min(4))))
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => md_blocks(children(n), "\n\n"),
    }
}

pub fn to_markdown(title: &str, json: &str) -> String {
    let doc = parse(json);
    let body = md_blocks(children(&doc), "\n\n");
    let mut out = String::new();
    if !title.trim().is_empty() {
        out.push_str(&format!("# {}\n\n", esc_md(title.trim())));
    }
    out.push_str(&body);
    out.push('\n');
    out
}

// ---------- HTML ----------

fn html_inline(nodes: &[Value]) -> String {
    let mut out = String::new();
    for n in nodes {
        match ty(n) {
            "text" => {
                let mut s = esc_html(n.get("text").and_then(Value::as_str).unwrap_or(""));
                for (m, tag) in [("code", "code"), ("bold", "strong"), ("italic", "em"), ("underline", "u"), ("strike", "s")] {
                    if has_mark(n, m) {
                        s = format!("<{tag}>{s}</{tag}>");
                    }
                }
                if let Some(url) = mark(n, "link").and_then(|m| attr_str(m, "href")).and_then(safe_url) {
                    s = format!("<a href=\"{}\">{s}</a>", esc_html(url));
                }
                out.push_str(&s);
            }
            "hardBreak" => out.push_str("<br>"),
            "mention" => out.push_str(&esc_html(attr_str(n, "label").unwrap_or(""))),
            _ => out.push_str(&html_inline(children(n))),
        }
    }
    out
}

fn html_blocks(nodes: &[Value]) -> String {
    nodes.iter().map(html_block).collect::<Vec<_>>().join("\n")
}

fn html_block(n: &Value) -> String {
    match ty(n) {
        "paragraph" => format!("<p>{}</p>", html_inline(children(n))),
        "heading" => {
            let l = n.get("attrs").and_then(|a| a.get("level")).and_then(Value::as_u64).unwrap_or(1).clamp(1, 6);
            format!("<h{l}>{}</h{l}>", html_inline(children(n)))
        }
        "blockquote" => format!("<blockquote>\n{}\n</blockquote>", html_blocks(children(n))),
        "codeBlock" => {
            let code: String = children(n).iter().filter_map(|c| c.get("text").and_then(Value::as_str)).collect();
            format!("<pre><code>{}</code></pre>", esc_html(&code))
        }
        "horizontalRule" => "<hr>".into(),
        "bulletList" => format!("<ul>\n{}\n</ul>", html_blocks(children(n))),
        "orderedList" => format!("<ol>\n{}\n</ol>", html_blocks(children(n))),
        "taskList" => format!("<ul class=\"tasks\">\n{}\n</ul>", html_blocks(children(n))),
        "listItem" => format!("<li>{}</li>", html_blocks(children(n))),
        "taskItem" => {
            let done = n.get("attrs").and_then(|a| a.get("checked")).and_then(Value::as_bool) == Some(true);
            format!(
                "<li><input type=\"checkbox\" disabled{}> {}</li>",
                if done { " checked" } else { "" },
                html_blocks(children(n))
            )
        }
        _ => html_blocks(children(n)),
    }
}

const CSS: &str = "body{font-family:Georgia,'Times New Roman',serif;max-width:42rem;margin:3rem auto;padding:0 1.25rem;line-height:1.65;color:#1c1c1a}\
h1,h2,h3{line-height:1.2}blockquote{margin-left:0;padding-left:1rem;border-left:3px solid #ccc;color:#555}\
pre{background:#f4f2ee;padding:.75rem 1rem;overflow:auto}code{font-family:ui-monospace,Menlo,monospace;font-size:.9em}\
ul.tasks{list-style:none;padding-left:0}img{max-width:100%}@media print{body{margin:0;max-width:none}}";

/// A standalone HTML page. With `print`, the page opens the print dialog on load so it
/// can be saved as a PDF from any browser.
pub fn to_html(title: &str, json: &str, print: bool) -> String {
    let doc = parse(json);
    let t = esc_html(title.trim());
    let heading = if t.is_empty() { String::new() } else { format!("<h1>{t}</h1>\n") };
    let script = if print { "<script>window.addEventListener('load',()=>window.print())</script>\n" } else { "" };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n<article>\n{heading}{}\n</article>\n{script}</body>\n</html>\n",
        if t.is_empty() { "Untitled" } else { &t },
        html_blocks(children(&doc))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"{"type":"doc","content":[
      {"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"Findings"}]},
      {"type":"paragraph","content":[
        {"type":"text","text":"Meridian got "},
        {"type":"text","text":"$4M","marks":[{"type":"bold"}]},
        {"type":"text","text":" see "},
        {"type":"mention","attrs":{"id":"src:3","label":"Contract.pdf"}},
        {"type":"text","text":" and ","marks":[]},
        {"type":"text","text":"site","marks":[{"type":"link","attrs":{"href":"https://x.org/a(b)"}}]},
        {"type":"text","text":" <b>","marks":[{"type":"link","attrs":{"href":"javascript:alert(1)"}}]}
      ]},
      {"type":"bulletList","content":[
        {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"one"}]},
          {"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"nested"}]}]}]}]},
        {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"two"}]}]}
      ]},
      {"type":"taskList","content":[
        {"type":"taskItem","attrs":{"checked":true},"content":[{"type":"paragraph","content":[{"type":"text","text":"done"}]}]},
        {"type":"taskItem","attrs":{"checked":false},"content":[{"type":"paragraph","content":[{"type":"text","text":"todo"}]}]}
      ]},
      {"type":"blockquote","content":[{"type":"paragraph","content":[{"type":"text","text":"quoted"}]}]},
      {"type":"codeBlock","attrs":{"language":"rs"},"content":[{"type":"text","text":"let a = 1;"}]}
    ]}"#;

    #[test]
    fn markdown_export() {
        let md = to_markdown("My *Story*", DOC);
        assert!(md.starts_with("# My \\*Story\\*\n\n## Findings\n\n"));
        assert!(md.contains("Meridian got **\\$4M**".replace("\\$", "$").as_str()));
        assert!(md.contains("Contract.pdf"));
        assert!(!md.contains("[[") && !md.contains("src:3"));
        assert!(md.contains("[site](https://x.org/a(b%29)"));
        assert!(!md.contains("javascript"));
        assert!(md.contains("- one\n  - nested\n- two"));
        assert!(md.contains("- [x] done\n- [ ] todo"));
        assert!(md.contains("> quoted"));
        assert!(md.contains("```rs\nlet a = 1;\n```"));
    }

    #[test]
    fn html_export_escapes_and_drops_unsafe_links() {
        let h = to_html("A & B", DOC, false);
        assert!(h.contains("<title>A &amp; B</title>"));
        assert!(h.contains("<strong>$4M</strong>"));
        assert!(h.contains("&lt;b&gt;"));
        assert!(!h.contains("javascript"));
        assert!(h.contains("<a href=\"https://x.org/a(b)\">site</a>"));
        assert!(h.contains("<input type=\"checkbox\" disabled checked>"));
        assert!(!h.contains("window.print"));
        assert!(to_html("x", DOC, true).contains("window.print"));
    }

    #[test]
    fn empty_and_invalid_content_do_not_panic() {
        assert_eq!(to_markdown("", ""), "\n");
        assert!(to_html("", "not json", false).contains("<title>Untitled</title>"));
    }
}
