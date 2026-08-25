//! Renders a Board as a markdown file in the format the Obsidian Kanban
//! community plugin expects: a frontmatter marker, then one "## list name"
//! heading per list with "- [ ] card name" items underneath.

use crate::json::{self, Json};
use crate::trello::{Board, Card, Label, List};

pub fn write(board: &Board) -> String {
    let mut out = String::new();
    out.push_str("<!-- exported from trello board: ");
    out.push_str(&board.name);
    out.push_str(" -->\n");
    out.push_str("---\n\nkanban-plugin: board\n\n---\n\n");

    for list in &board.lists {
        out.push_str("## ");
        out.push_str(&list.name);
        out.push_str("\n\n");
        for card in &list.cards {
            out.push_str("- [ ] ");
            out.push_str(&single_line(&card.name));
            out.push('\n');
            if !card.labels.is_empty() {
                out.push_str("  %%labels:");
                out.push_str(&json::stringify(&labels_to_json(&card.labels)));
                out.push_str("%%\n");
            }
            if !card.desc.trim().is_empty() {
                for line in card.desc.lines() {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        out.push('\n');
    }

    out
}

fn single_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

/// Reads a markdown file written by `write` (or hand-edited in the same
/// shape) back into a Board. Only the fields `write` produces round-trip:
/// board name, list names, card names, card descriptions, and labels (kept
/// in a hidden `%%labels:...%%` line so they don't show up in the rendered
/// board).
pub fn parse(markdown: &str) -> Result<Board, String> {
    let mut board_name = "Untitled Board".to_string();
    let mut lists: Vec<List> = Vec::new();

    for line in markdown.lines() {
        if let Some(rest) = line.strip_prefix("<!-- exported from trello board: ") {
            if let Some(name) = rest.strip_suffix(" -->") {
                board_name = name.to_string();
            }
            continue;
        }
        if line.trim().is_empty() || line == "---" || line == "kanban-plugin: board" {
            continue;
        }
        if let Some(name) = line.strip_prefix("## ") {
            lists.push(List { name: name.trim().to_string(), cards: Vec::new() });
            continue;
        }
        if let Some(rest) = line.strip_prefix("- [ ] ").or_else(|| line.strip_prefix("- [x] ")) {
            let list = lists
                .last_mut()
                .ok_or_else(|| "found a card before any list heading".to_string())?;
            list.cards.push(Card { name: rest.to_string(), desc: String::new(), labels: Vec::new() });
            continue;
        }
        if let Some(rest) = line.strip_prefix("  %%labels:").and_then(|r| r.strip_suffix("%%")) {
            let list = lists
                .last_mut()
                .ok_or_else(|| "found a labels line before any list heading".to_string())?;
            let card = list
                .cards
                .last_mut()
                .ok_or_else(|| "found a labels line before any card".to_string())?;
            let parsed = json::parse(rest)
                .map_err(|e| format!("invalid %%labels%% metadata: {}", e))?;
            card.labels = json_to_labels(&parsed);
            continue;
        }
        if let Some(rest) = line.strip_prefix("  ") {
            let list = lists
                .last_mut()
                .ok_or_else(|| "found a description line before any list heading".to_string())?;
            let card = list
                .cards
                .last_mut()
                .ok_or_else(|| "found a description line before any card".to_string())?;
            if !card.desc.is_empty() {
                card.desc.push('\n');
            }
            card.desc.push_str(rest);
            continue;
        }
        return Err(format!("unrecognized line: \"{}\"", line));
    }

    Ok(Board { name: board_name, lists })
}

fn labels_to_json(labels: &[Label]) -> Json {
    Json::Array(
        labels
            .iter()
            .map(|label| {
                Json::Object(vec![
                    ("name".to_string(), Json::String(label.name.clone())),
                    ("color".to_string(), Json::String(label.color.clone())),
                ])
            })
            .collect(),
    )
}

fn json_to_labels(value: &Json) -> Vec<Label> {
    value
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|item| Label {
            name: item.get("name").and_then(Json::as_str).unwrap_or("").to_string(),
            color: item.get("color").and_then(Json::as_str).unwrap_or("").to_string(),
        })
        .collect()
}
