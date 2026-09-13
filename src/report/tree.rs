use crate::analysis::file_icon_for;
use crate::cli::Config;
use crate::model::{Kind, SortOrder, TreeNode};
use crate::util::{bidi_safe, extension_of, human_size};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";

struct TreeChars {
    branch: &'static str,
    last: &'static str,
    pipe: &'static str,
    blank: &'static str,
    dir_icon: &'static str,
    file_icon: &'static str,
    link_icon: &'static str,
}

const UNICODE: TreeChars = TreeChars {
    branch: "├── ",
    last: "└── ",
    pipe: "│   ",
    blank: "    ",
    dir_icon: "📁 ",
    file_icon: "📄 ",
    link_icon: "🔗 ",
};
const ASCII: TreeChars = TreeChars {
    branch: "|-- ",
    last: "`-- ",
    pipe: "|   ",
    blank: "    ",
    dir_icon: "[D] ",
    file_icon: "[F] ",
    link_icon: "[L] ",
};

pub fn render(node: &TreeNode, cfg: &Config) -> String {
    let chars = if cfg.ascii { &ASCII } else { &UNICODE };
    let mut out = String::new();
    let root_label = format!(
        "{} ({} items, {})",
        bidi_safe(&cfg.root.display().to_string()),
        node.item_count,
        human_size(node.total_size)
    );
    out.push_str(&paint(&root_label, &[BOLD, BLUE], cfg.color));
    out.push('\n');
    for (i, child) in node.children.iter().enumerate() {
        render_node(
            child,
            "",
            i + 1 == node.children.len(),
            chars,
            cfg,
            &mut out,
        );
    }
    out
}

fn render_node(
    node: &TreeNode,
    prefix: &str,
    is_last: bool,
    chars: &TreeChars,
    cfg: &Config,
    out: &mut String,
) {
    let connector = if is_last { chars.last } else { chars.branch };
    let line = match node.kind {
        Kind::Dir => {
            let name = paint(&bidi_safe(&node.name), &[BOLD, BLUE], cfg.color);
            let mut label = format!("{}{}", chars.dir_icon, name);
            if node.item_count > 0 {
                label.push_str(&paint(
                    &format!(
                        "  ({} items, {})",
                        node.item_count,
                        human_size(node.total_size)
                    ),
                    &[DIM, GRAY],
                    cfg.color,
                ));
            }
            if node.truncated {
                label.push_str(&paint("  [depth limit reached]", &[DIM, YELLOW], cfg.color));
            }
            if node.hidden_by_min_size > 0 {
                label.push_str(&paint(
                    &format!("  [+{} smaller files hidden]", node.hidden_by_min_size),
                    &[DIM, GRAY],
                    cfg.color,
                ));
            }
            label
        }
        Kind::File => {
            let icon = if cfg.ascii {
                chars.file_icon
            } else {
                file_icon_for(&extension_of(std::path::Path::new(&node.name)))
            };
            let size = paint(
                &format!("({})", human_size(node.own_size)),
                &[DIM, GRAY],
                cfg.color,
            );
            format!("{icon}{}  {size}", bidi_safe(&node.name))
        }
        Kind::Symlink => format!(
            "{}{}  {}",
            chars.link_icon,
            paint(&bidi_safe(&node.name), &[CYAN], cfg.color),
            paint("(symlink)", &[DIM, GRAY], cfg.color)
        ),
        Kind::Other => format!("{}{}", chars.file_icon, bidi_safe(&node.name)),
    };
    out.push_str(&format!("{prefix}{connector}{line}\n"));
    if node.kind == Kind::Dir && !node.children.is_empty() {
        let child_prefix = format!("{prefix}{}", if is_last { chars.blank } else { chars.pipe });
        for (i, child) in node.children.iter().enumerate() {
            render_node(
                child,
                &child_prefix,
                i + 1 == node.children.len(),
                chars,
                cfg,
                out,
            );
        }
    }
}

fn paint(s: &str, codes: &[&str], enabled: bool) -> String {
    if !enabled || s.is_empty() {
        s.to_string()
    } else {
        format!("{}{}{}", codes.concat(), s, RESET)
    }
}

#[allow(dead_code)]
fn _sort_marker(_: SortOrder) {}
