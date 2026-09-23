//! 第 12 课：[The full TUI](https://www.byoharness.dev/chapters/12-full-tui.html)
//!
//! 第 08 课输入框是一次性的：收一行就退，循环继续 `println!`。
//! 本课一个程序占整屏：视口、边框输入、审批、转圈、飞行中的子 agent。
//!
//! 课上是 Bubble Tea。本仓库继续 crossterm MVU，不上 ratatui。
//!
//! ```text
//! 空闲 ──提交──▶ 在跑 ──Confirm──▶ 等审批 ──y/n──▶ 在跑 ──结束──▶ 空闲
//! ```
//!
//! 循环在后台线程跑。`println!` 进管子变成视口行。
//! 审批走通道：线程堵着等，主循环按 y/n 写回。循环不认识 TUI。
//!
//! `agent` 不再 import `ui`（转圈交给状态行），所以
//! `ui → subagent → agent` 不成环。
//!
//! 管道 / `BYO_PLAIN_INPUT=1` 仍走 `stdin.lines()`。备用屏只给 TTY。
//!
//! 启动：`bash scripts/run.sh 12`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::ui::{
        ChatKey, Harness, HarnessCmd, HarnessEvent, ModelState,
    };

    fn harness() -> Harness {
        Harness::new(40, 14, "banner\n".into(), Vec::new())
    }

    #[test]
    fn lesson_12_idle_submit_runs() {
        let mut h = harness();
        let _ = h.apply(HarnessEvent::Key(ChatKey::Char('a')));
        let cmd = h.apply(HarnessEvent::Key(ChatKey::Enter));
        assert_eq!(cmd, HarnessCmd::Submit("a".into()));
        assert_eq!(h.state, ModelState::Running);
    }

    #[test]
    fn lesson_12_approval_is_one_key() {
        let mut h = harness();
        h.state = ModelState::Running;
        let _ = h.apply(HarnessEvent::Approval {
            prompt: "approve?".into(),
        });
        assert_eq!(h.state, ModelState::AwaitingApproval);
        assert_eq!(
            h.apply(HarnessEvent::Key(ChatKey::Char('n'))),
            HarnessCmd::Reply(false)
        );
        assert_eq!(h.state, ModelState::Running);
    }

    #[test]
    fn lesson_12_scroll_up_stops_follow() {
        let mut h = harness();
        let _ = h.apply(HarnessEvent::Append(
            (0..20).map(|i| format!("L{i}\n")).collect(),
        ));
        assert!(h.follow_bottom);
        let _ = h.apply(HarnessEvent::PageUp);
        assert!(!h.follow_bottom);
        let _ = h.apply(HarnessEvent::End);
        assert!(h.follow_bottom);
    }

    #[test]
    fn lesson_12_status_lists_active_subagent() {
        let mut h = harness();
        h.state = ModelState::Running;
        let line = h.status_line(&[("research".into(), 1)]);
        assert!(line.contains("research"), "{line}");
    }
}
