mod json;
mod obsidian;
mod trello;

use std::env;
use std::fs;
use std::process;

struct Args {
    input: String,
    output: String,
    json_output: bool,
    reverse: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut json_output = false;
    let mut reverse = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--json" => json_output = true,
            "--reverse" => reverse = true,
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            _ => positional.push(arg),
        }
    }
    if positional.len() != 2 {
        return Err("expected exactly two positional arguments: <input> <output>".to_string());
    }
    Ok(Args { input: positional.remove(0), output: positional.remove(0), json_output, reverse })
}

fn print_usage() {
    eprintln!("kanban-bridge - convert between Trello JSON board exports and Obsidian Kanban markdown");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  kanban-bridge <input.json> <output.md> [--json]      trello -> obsidian");
    eprintln!("  kanban-bridge <input.md> <output.json> --reverse     obsidian -> trello");
    eprintln!();
    eprintln!("  --json      print the conversion summary as a JSON object instead of plain text");
    eprintln!("  --reverse   convert an Obsidian Kanban file back into a Trello export");
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("error: {}", msg);
            print_usage();
            process::exit(2);
        }
    };

    let raw = match fs::read_to_string(&args.input) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("error: could not read {}: {}", args.input, e);
            process::exit(1);
        }
    };

    if args.reverse {
        run_reverse(&args, &raw);
        return;
    }

    let parsed = match json::parse(&raw) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("error: could not parse {} as JSON: {}", args.input, e);
            process::exit(1);
        }
    };

    let (board, stats) = match trello::import(&parsed) {
        Ok(result) => result,
        Err(msg) => {
            eprintln!("error: {} does not look like a Trello board export: {}", args.input, msg);
            process::exit(1);
        }
    };

    let markdown = obsidian::write(&board);

    if let Err(e) = fs::write(&args.output, markdown) {
        eprintln!("error: could not write {}: {}", args.output, e);
        process::exit(1);
    }

    let list_count = board.lists.len();
    let card_count: usize = board.lists.iter().map(|l| l.cards.len()).sum();

    if args.json_output {
        print_json_report(&args, list_count, card_count, &stats);
    } else {
        print_text_report(&args, list_count, card_count, &stats);
    }
}

fn run_reverse(args: &Args, markdown: &str) {
    let board = match obsidian::parse(markdown) {
        Ok(board) => board,
        Err(msg) => {
            eprintln!("error: {} does not look like an Obsidian Kanban file: {}", args.input, msg);
            process::exit(1);
        }
    };

    let list_count = board.lists.len();
    let card_count: usize = board.lists.iter().map(|l| l.cards.len()).sum();

    let exported = trello::export(&board);
    let json_text = json::stringify(&exported);

    if let Err(e) = fs::write(&args.output, json_text) {
        eprintln!("error: could not write {}: {}", args.output, e);
        process::exit(1);
    }

    if args.json_output {
        println!(
            "{{\"input\":\"{}\",\"output\":\"{}\",\"lists_converted\":{},\"cards_converted\":{}}}",
            json::escape(&args.input),
            json::escape(&args.output),
            list_count,
            card_count
        );
    } else {
        println!("converted {} -> {}", args.input, args.output);
        println!("  lists converted: {}", list_count);
        println!("  cards converted: {}", card_count);
    }
}

fn print_text_report(args: &Args, list_count: usize, card_count: usize, stats: &trello::ImportStats) {
    println!("converted {} -> {}", args.input, args.output);
    println!("  lists converted: {}", list_count);
    println!("  cards converted: {}", card_count);
    if stats.closed_lists_skipped > 0 {
        println!("  archived lists skipped: {}", stats.closed_lists_skipped);
    }
    if stats.closed_cards_skipped > 0 {
        println!("  archived cards skipped: {}", stats.closed_cards_skipped);
    }
    if stats.orphaned_cards_skipped > 0 {
        println!("  orphaned cards skipped: {}", stats.orphaned_cards_skipped);
    }
    for warning in &stats.warnings {
        println!("  warning: {}", warning);
    }
}

fn print_json_report(args: &Args, list_count: usize, card_count: usize, stats: &trello::ImportStats) {
    let warnings_json: Vec<String> = stats
        .warnings
        .iter()
        .map(|w| format!("\"{}\"", json::escape(w)))
        .collect();
    println!(
        "{{\"input\":\"{}\",\"output\":\"{}\",\"lists_converted\":{},\"cards_converted\":{},\"closed_lists_skipped\":{},\"closed_cards_skipped\":{},\"orphaned_cards_skipped\":{},\"warnings\":[{}]}}",
        json::escape(&args.input),
        json::escape(&args.output),
        list_count,
        card_count,
        stats.closed_lists_skipped,
        stats.closed_cards_skipped,
        stats.orphaned_cards_skipped,
        warnings_json.join(",")
    );
}
