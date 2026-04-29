use chrono::Local;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn get_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Some(branch);
        }
    }
    None
}

fn build_prompt(last_success: bool, exit_code: i32) -> String {
    let username = env::var("USER").unwrap_or_else(|_| "user".into());
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "host".into());

    let cwd = env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".into());
    let home = env::var("HOME").unwrap_or_default();
    let display_path = cwd.replace(&home, "~");
    let now = Local::now().format("%H:%M:%S").to_string();
    let git_info = if let Some(branch) = get_git_branch() {
        format!(" on {}:{}", "git".blue(), branch.cyan())
    } else {
        "".to_string()
    };

    let status_icon = if last_success {
        "o".green().bold()
    } else {
        format!("x({})", exit_code).red().bold()
    };

    format!(
        "{} {} {} {} in {}{} {} [{}] drash\n{}",
        "#".blue(),
        username.cyan().bold(),
        "@".blue(),
        hostname.green(),
        display_path.yellow().bold(),
        git_info,
        status_icon,
        now.white(),
        "$ ".red()
    )
}

fn get_history_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".drash_history");
    path
}

pub fn start_repl() -> rustyline::Result<()> {
    let mut rl = DefaultEditor::new()?;
    let history_path = get_history_path();
    let _ = rl.load_history(&history_path);
    let mut last_success = true;
    let mut exit_code = 0;
    loop {
        let readline = rl.readline(&build_prompt(last_success, exit_code));
        match readline {
            Ok(line) => {
                if let Some(args) = shlex::split(&line) {
                    if args.is_empty() {
                        continue;
                    }
                    let cmd = &args[0];
                    let params = &args[1..];
                    if cmd == "exit" {
                        println!("再見！");
                        break;
                    }
                    rl.add_history_entry(line.as_str()).unwrap();
                    //TODO: 這裡可以加入內建指令的處理，例如 cd、export 等，這些指令不會啟動子程序，而是直接在 Shell 內部執行。
                    let child = std::process::Command::new(cmd).args(params).spawn();

                    match child {
                        Ok(mut handle) => {
                            let status = handle.wait().unwrap();
                            exit_code = status.code().unwrap_or(-1);
                            last_success = status.success();
                        }
                        Err(_) => {
                            eprintln!("drash: command not found: {}", args[0]);
                            exit_code = 127;
                            last_success = false;
                        }
                    }
                } else {
                    println!("語法錯誤：引號未閉合");
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    if let Err(e) = rl.save_history(&history_path) {
        eprintln!("無法儲存歷史紀錄至 {:?}: {}", history_path, e);
    }
    Ok(())
}
