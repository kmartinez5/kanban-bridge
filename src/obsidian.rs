//! Renders a Board as a markdown file in the format the Obsidian Kanban
//! community plugin expects: a frontmatter marker, then one "## list name"
//! heading per list with "- [ ] card name" items underneath.

use crate::trello::Board;

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
