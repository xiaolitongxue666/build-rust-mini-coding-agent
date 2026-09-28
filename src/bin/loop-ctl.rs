//! 后学 `/goal` / `/loop` 的文件面。scripts 调这里，不改 `agent_loop`。

use std::env;
use std::process;

use build_rust_mini_coding_agent::graph_ctl::{
    active_thread, after_implement, after_merge, after_verify, graph_maker_prompt,
    graph_status_text, graph_stop_exists, parse_thread, read_checkpoint, should_run,
    write_checkpoint,
};
use build_rust_mini_coding_agent::loop_ctl::{
    advance, cwd_root, goal_status_text, goal_stop_exists, maker_prompt, patrol_status_text,
    read_goal, read_patrol, read_state, should_continue, write_state,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let root = cwd_root()?;
    match cmd.as_str() {
        "should-continue" => {
            let spec = read_goal(&root)?;
            let state = read_state(&root)?;
            if should_continue(&state, &spec, goal_stop_exists(&root)) {
                Ok(())
            } else {
                process::exit(1);
            }
        }
        "maker-prompt" => {
            print!("{}", maker_prompt(&root)?);
            Ok(())
        }
        "verify-command" => {
            print!("{}", read_goal(&root)?.verify_command.trim());
            Ok(())
        }
        "advance" => {
            let mut exit_code = 1;
            let mut maker = String::new();
            let mut verify = String::new();
            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--exit-code" => {
                        exit_code = args
                            .next()
                            .ok_or("missing --exit-code")?
                            .parse::<i32>()
                            .map_err(|err| err.to_string())?;
                    }
                    "--maker-file" => {
                        maker = read_optional(&args.next().ok_or("missing --maker-file")?)?;
                    }
                    "--verify-file" => {
                        verify = read_optional(&args.next().ok_or("missing --verify-file")?)?;
                    }
                    other => return Err(format!("unknown flag: {other}")),
                }
            }
            let spec = read_goal(&root)?;
            let state = read_state(&root)?;
            let notes = if verify.is_empty() {
                maker
            } else {
                format!("{maker}\n\n--- verify ---\n{verify}")
            };
            let next = advance(&state, &spec, exit_code, &notes);
            write_state(&root, &next)?;
            println!("{}", next.status.as_str());
            Ok(())
        }
        "interval-seconds" => {
            print!("{}", read_patrol(&root)?.interval_secs);
            Ok(())
        }
        "patrol-prompt" => {
            print!("{}", read_patrol(&root)?.prompt.trim());
            Ok(())
        }
        "status" => {
            print!("{}", goal_status_text(&root));
            print!("{}", patrol_status_text(&root));
            Ok(())
        }
        "graph-should-continue" => {
            let thread = graph_thread(&root);
            let state = read_checkpoint(&root, &thread)?;
            if should_run(&state, graph_stop_exists(&root, &thread)) {
                Ok(())
            } else {
                process::exit(1);
            }
        }
        "graph-current" => {
            let thread = graph_thread(&root);
            print!("{}", read_checkpoint(&root, &thread)?.current.as_str());
            Ok(())
        }
        "graph-maker-prompt" => {
            let thread = graph_thread(&root);
            print!("{}", graph_maker_prompt(&root, &thread)?);
            Ok(())
        }
        "graph-status" => {
            let thread = graph_thread(&root);
            print!("{}", graph_status_text(&root, &thread));
            Ok(())
        }
        "graph-step" => {
            let step = args
                .next()
                .ok_or("usage: loop-ctl graph-step implement|verify|merge")?;
            let thread = graph_thread(&root);
            let mut notes = String::new();
            let mut exit_code = 1;
            let rest: Vec<String> = args.collect();
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--notes-file" => {
                        i += 1;
                        notes = read_optional(rest.get(i).ok_or("missing --notes-file")?)?;
                    }
                    "--verify-file" => {
                        i += 1;
                        let extra = read_optional(rest.get(i).ok_or("missing --verify-file")?)?;
                        if !extra.is_empty() {
                            notes = format!("{notes}\n\n--- verify ---\n{extra}");
                        }
                    }
                    "--exit-code" => {
                        i += 1;
                        exit_code = rest
                            .get(i)
                            .ok_or("missing --exit-code")?
                            .parse::<i32>()
                            .map_err(|err| err.to_string())?;
                    }
                    other => return Err(format!("unknown flag: {other}")),
                }
                i += 1;
            }
            let state = read_checkpoint(&root, &thread)?;
            let next = match step.as_str() {
                "implement" => after_implement(&state, &notes),
                "verify" => after_verify(&state, &read_goal(&root)?, exit_code),
                "merge" => after_merge(&state),
                other => return Err(format!("unknown graph step: {other}")),
            };
            write_checkpoint(&root, &next)?;
            println!("{}", next.current.as_str());
            Ok(())
        }
        "" | "help" | "-h" | "--help" => {
            println!(
                "usage: loop-ctl should-continue | maker-prompt | verify-command | advance | interval-seconds | patrol-prompt | status | graph-should-continue | graph-current | graph-maker-prompt | graph-status | graph-step"
            );
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn graph_thread(root: &std::path::Path) -> String {
    match env::var("GRAPH_THREAD") {
        Ok(raw) if !raw.trim().is_empty() => {
            parse_thread(&raw).unwrap_or_else(|_| active_thread(root))
        }
        _ => active_thread(root),
    }
}

fn read_optional(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).or_else(|_| Ok(String::new()))
}
