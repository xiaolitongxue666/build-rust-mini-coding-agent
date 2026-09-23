//! 第 04 课：启动字标。ANSI Shadow，宽屏大图，窄屏或非 TTY 单行。
//! 课上是 BETTATECH；本仓字标是 RUSTBYO。ANSI 只打在启动时，不准进 system prompt。

use std::io::{self, IsTerminal, Write};

const BOLD_CYAN: &str = "\x1b[1;36m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

/// 字标视觉宽度 66，留 3 列呼吸空间（课上 75 列字标用 78）。
pub const BIG_BANNER_MIN_WIDTH: usize = 69;
pub const BIG_BANNER_WIDTH: usize = 66;

const BIG_BANNER: &str = "\
██████╗  ██╗   ██╗ ███████╗ ████████╗ ██████╗  ██╗   ██╗  ██████╗ \n\
██╔══██╗ ██║   ██║ ██╔════╝ ╚══██╔══╝ ██╔══██╗ ╚██╗ ██╔╝ ██╔═══██╗\n\
██████╔╝ ██║   ██║ ███████╗    ██║    ██████╔╝  ╚████╔╝  ██║   ██║\n\
██╔══██╗ ██║   ██║ ╚════██║    ██║    ██╔══██╗   ╚██╔╝   ██║   ██║\n\
██║  ██║ ╚██████╔╝ ███████║    ██║    ██████╔╝    ██║    ╚██████╔╝\n\
╚═╝  ╚═╝  ╚═════╝  ╚══════╝    ╚═╝    ╚═════╝     ╚═╝     ╚═════╝ ";

/// stdout 不是 TTY（管道、重定向）时当 0 列，走小字标。
pub fn term_width() -> usize {
    if !io::stdout().is_terminal() {
        return 0;
    }
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(0)
}

pub fn banner_text(width: usize) -> String {
    let width = if width == 0 {
        BIG_BANNER_MIN_WIDTH
    } else {
        width
    };
    if width >= BIG_BANNER_MIN_WIDTH {
        format!(
            "{BOLD_CYAN}{BIG_BANNER}{RESET}\n{DIM} rust mini coding agent · DeepSeek{RESET}\n{DIM} type a message · /help · esc cancels a turn · /exit or ctrl-c twice to leave{RESET}\n\n"
        )
    } else {
        format!(
            "\n{BOLD_CYAN} RUSTBYO{RESET}{DIM} · rust mini coding agent · DeepSeek{RESET}\n{DIM} type a message · esc cancels a turn{RESET}\n\n"
        )
    }
}

pub fn print_banner() {
    let _ = io::stdout().write_all(banner_text(term_width()).as_bytes());
    let _ = io::stdout().flush();
}
