//! Pulls a board's lists and cards out of a parsed Trello export,
//! dropping archived lists/cards and anything the export links together
//! by an id that no longer resolves.

use crate::json::Json;

pub struct Board {
    pub name: String,
    pub lists: Vec<List>,
}

pub struct List {
    pub name: String,
    pub cards: Vec<Card>,
}

pub struct Card {
    pub name: String,
    pub desc: String,
}

#[derive(Default)]
pub struct ImportStats {
    pub closed_lists_skipped: usize,
    pub closed_cards_skipped: usize,
    pub orphaned_cards_skipped: usize,
    pub warnings: Vec<String>,
}

struct OpenList {
    id: String,
    name: String,
    pos: f64,
}

pub fn import(root: &Json) -> Result<(Board, ImportStats), String> {
    let board_name = root
        .get("name")
        .and_then(Json::as_str)
        .unwrap_or("Untitled Board")
        .to_string();
    let mut stats = ImportStats::default();

    let lists_json = root
        .get("lists")
        .and_then(Json::as_array)
        .ok_or_else(|| "missing a \"lists\" array".to_string())?;

    let mut open_lists: Vec<OpenList> = Vec::new();
    for item in lists_json {
        if item.get("closed").and_then(Json::as_bool).unwrap_or(false) {
            stats.closed_lists_skipped += 1;
            continue;
        }
        let id = item
            .get("id")
            .and_then(Json::as_str)
            .ok_or_else(|| "a list entry is missing \"id\"".to_string())?
            .to_string();
        let name = item
            .get("name")
            .and_then(Json::as_str)
            .unwrap_or("Untitled List")
            .to_string();
        let pos = item.get("pos").and_then(Json::as_f64).unwrap_or(0.0);
        open_lists.push(OpenList { id, name, pos });
    }
    open_lists.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap_or(std::cmp::Ordering::Equal));

    let cards_json = root
        .get("cards")
        .and_then(Json::as_array)
        .ok_or_else(|| "missing a \"cards\" array".to_string())?;

    // (card pos, card) per list, so each list's cards can be reordered
    // by their trello "pos" once every card has been grouped.
    let mut grouped: Vec<Vec<(f64, Card)>> = open_lists.iter().map(|_| Vec::new()).collect();

    for item in cards_json {
        if item.get("closed").and_then(Json::as_bool).unwrap_or(false) {
            stats.closed_cards_skipped += 1;
            continue;
        }
        let name = item
            .get("name")
            .and_then(Json::as_str)
            .unwrap_or("Untitled Card")
            .to_string();
        let id_list = match item.get("idList").and_then(Json::as_str) {
            Some(id) => id,
            None => {
                stats.warnings.push(format!("card \"{}\" has no idList; skipped", name));
                stats.orphaned_cards_skipped += 1;
                continue;
            }
        };
        let list_index = match open_lists.iter().position(|l| l.id == id_list) {
            Some(index) => index,
            None => {
                stats.warnings.push(format!(
                    "card \"{}\" references list id \"{}\" which is archived or missing; skipped",
                    name, id_list
                ));
                stats.orphaned_cards_skipped += 1;
                continue;
            }
        };
        let desc = item.get("desc").and_then(Json::as_str).unwrap_or("").to_string();
        let pos = item.get("pos").and_then(Json::as_f64).unwrap_or(0.0);
        grouped[list_index].push((pos, Card { name, desc }));
    }

    let lists: Vec<List> = open_lists
        .into_iter()
        .zip(grouped.into_iter())
        .map(|(open_list, mut cards)| {
            cards.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            List {
                name: open_list.name,
                cards: cards.into_iter().map(|(_, card)| card).collect(),
            }
        })
        .collect();

    Ok((Board { name: board_name, lists }, stats))
}
