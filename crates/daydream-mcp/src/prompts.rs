//! The server's instructions and its prompts: how an agent should go about making content,
//! learned from building a whole area through this server (`docs/tele-arena-area-retro.md`).
//!
//! Clients show prompts as commands (Claude Code: `/mcp__daydream__new_area`); each fills in
//! the user's idea and walks the same steps.

use serde_json::{Value, json};

/// The server's `instructions`, which a client puts in front of the agent.
pub const INSTRUCTIONS: &str = "\
Daydream edits a MajorMUD realm in Nightmare Redux's terms (tables, columns and keys as NMR names them).

Reading: list_tables, describe_table, list_rows, get_row, get_message, get_map, free_numbers and find_references. \
describe_table gives each column's choice list, what it refers to, and notes on what the game does with it: read \
them before writing a table. get_message gives a message's lines as the game reads them, with every record that uses \
it. game_status says what is open, whether edits go to the game's own files or a working copy, and whether the game \
is running.

Changing: preview_edits, show the user what will change, then apply_edits with its token. Give every batch of one \
piece of work the same changeset label, so undo with that changeset takes the whole piece back. Each applied batch \
keeps its edits in the realm's undo journal.

Checking: check_world reads the wiring across the world, as the editors' Home does: prices that are not what the \
words say, likely typos, placeholders left in, messages that read wrong (a line naming another creature, an arrival \
line the wrong way round), rooms that spawn nothing and monsters nothing brings. After a batch, run it on what you \
made (table and key) and tell the user what it finds.

How to work well:
- Before copying an existing row as a template, get_row it in full: stock rows hold junk in unused columns and \
placeholder numbers (a monster with messages 1, 1, 1).
- When a meaning or convention is inferred (an index base, what a column does in an unusual room type), check it \
against one known example before building on it, and tell the user what is checked and what is a guess.
- Build one of each kind first (one room, one monster, one item), apply it, and ask the user to try it in the game \
before making many.
- Message rows are three-line bundles shared between monsters, items and spells; a line may name the original \
creature. get_message shows what each line reads as and every record that uses it: read it before reusing or \
changing a message, since a change reaches every user. To copy a monster with messages of its own, clone it with \
\"messages\": \"named\".
- Report progress to the user in milestones they can see, not tool by tool.";

/// The steps every prompt ends with.
const STEPS: &str = "\
Work in this order:
1. Call game_status, then read the tables involved (describe_table, with its notes) and the existing rows most like \
what is wanted (list_rows, get_row in full). Note junk values and placeholder numbers in them, and read \
who else uses anything you mean to share (get_message for a message, find_references for the rest).
2. Write a short plan for the user: the numbers to use (free_numbers), the rows to add or change, and anything \
inferred rather than checked. Wait for the user's go ahead.
3. Make one of each kind first under a changeset label, preview it, show the user, apply it, and ask the user to \
try it in the game.
4. Only then make the rest, in batches under the same changeset, previewing each.
5. Run check_world on the records you made, and fix or explain what it finds.
6. Finish with what was built, its numbers, the changeset to undo it with, and what the user should test.";

struct Prompt {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    /// The arguments: name, description, required.
    args: &'static [(&'static str, &'static str, bool)],
    /// What is asked, with `{name}` for each argument.
    ask: &'static str,
}

const PROMPTS: &[Prompt] = &[
    Prompt {
        name: "new_area",
        title: "Build a new area",
        description: "Rooms, monsters, items and shops for a new area, built in small tested steps.",
        args: &[
            (
                "idea",
                "What the area is: its theme, size, level range, and how players reach it",
                true,
            ),
            ("map", "The map number to build it on, if known", false),
        ],
        ask: "Build a new area in this realm: {idea}\nMap: {map}",
    },
    Prompt {
        name: "new_monster",
        title: "Add a monster",
        description: "A new monster with its own messages, placed where it should spawn.",
        args: &[
            (
                "idea",
                "The monster: what it is, its level, and where it should appear",
                true,
            ),
            (
                "like",
                "An existing monster to start from, by name or number",
                false,
            ),
        ],
        ask: "Add a monster to this realm: {idea}\nStart from: {like}\n\
              Give it its own hit, move and death message rows that name it, rather than borrowing \
              another monster's (a clone with \"messages\": \"named\" copies the ones that name its \
              template, with the name swapped).",
    },
    Prompt {
        name: "new_item",
        title: "Add an item",
        description: "A new item, and where players get it (a room, a shop or a monster's drop).",
        args: &[
            (
                "idea",
                "The item: what it is, what it does, and how players get it",
                true,
            ),
            (
                "like",
                "An existing item to start from, by name or number",
                false,
            ),
        ],
        ask: "Add an item to this realm: {idea}\nStart from: {like}",
    },
];

/// `prompts/list`.
pub fn list() -> Value {
    let prompts: Vec<Value> = PROMPTS
        .iter()
        .map(|p| {
            let args: Vec<Value> = p
                .args
                .iter()
                .map(|(name, description, required)| {
                    json!({ "name": name, "description": description, "required": required })
                })
                .collect();
            json!({
                "name": p.name,
                "title": p.title,
                "description": p.description,
                "arguments": args,
            })
        })
        .collect();
    json!({ "prompts": prompts })
}

/// `prompts/get`: the prompt's message, or why it cannot be given.
pub fn get(params: &Value) -> Result<Value, String> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let p = PROMPTS
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("no prompt {name}"))?;
    let given = params.get("arguments");
    let mut text = p.ask.to_string();
    for (arg, _, required) in p.args {
        let value = given
            .and_then(|a| a.get(*arg))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty());
        if value.is_none() && *required {
            return Err(format!("{name} needs {arg}"));
        }
        text = text.replace(&format!("{{{arg}}}"), value.unwrap_or("(not given)"));
    }
    Ok(json!({
        "description": p.description,
        "messages": [{
            "role": "user",
            "content": { "type": "text", "text": format!("{text}\n\n{STEPS}") }
        }]
    }))
}
