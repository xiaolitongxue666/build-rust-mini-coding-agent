//! 第 04 课：[UI polish](https://www.byoharness.dev/chapters/04-ui-polish.html)
//! banner / spinner / 薄输入。不写第 05 斜杠命令，不上 Bubble Tea。

mod banner;
mod input;
mod spinner;

pub use banner::{banner_text, print_banner, term_width, BIG_BANNER_MIN_WIDTH, BIG_BANNER_WIDTH};
pub use input::{ctrl_c_action, CtrlCAction, LineResult, PromptRead, ReplLine, SessionInput};
pub use spinner::{spin_until, stdout_is_tty, Wait};
