use chrono::Local;
use colored::*;
use rustyline::{
    completion::{Completer, FilenameCompleter, Pair},
    error::ReadlineError,
    highlight::MatchingBracketHighlighter,
    hint::HistoryHinter,
    history::DefaultHistory,
    validate::MatchingBracketValidator,
    Editor, Helper, Highlighter, Hinter, Validator,
};
use std::{
    collections::BTreeSet, env, fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command,
};

#[derive(Helper, Hinter, Highlighter, Validator)]
pub struct DrashHelper {
    completer: FilenameCompleter,
    #[rustyline(Highlighter)]
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    pub system_commands: BTreeSet<String>,
}

impl Completer for DrashHelper {
    type Candidate = Pair;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let slice = &line[..pos];
        if !slice.contains(' ') {
            let mut candidates: Vec<Pair> = Vec::new();
            let builtins = ["cd", "exit", "export", "history", "help"];
            for cmd in builtins {
                if cmd.starts_with(slice) {
                    candidates.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }

            for cmd in self.system_commands.range(slice.to_string()..) {
                if !cmd.starts_with(slice) {
                    break;
                }
                candidates.push(Pair {
                    display: cmd.clone(),
                    replacement: cmd.clone(),
                });
            }

            if !candidates.is_empty() {
                return Ok((0, candidates));
            }
        }
        self.completer.complete(line, pos, ctx)
    }
}

impl DrashHelper {
    pub fn new() -> Self {
        let mut commands = BTreeSet::new();
        if let Ok(path_var) = env::var("PATH") {
            for dir in path_var.split(':') {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Ok(metadata) = path.metadata() {
                                if metadata.permissions().mode() & 0o111 != 0 {
                                    if let Some(name) = path.file_name() {
                                        commands.insert(name.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Self {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter::new(),
            validator: MatchingBracketValidator::new(),
            system_commands: commands,
        }
    }
}

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
    let h = DrashHelper::new();
    let mut rl: Editor<DrashHelper, DefaultHistory> = Editor::new()?;
    rl.set_helper(Some(h));
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
                    rl.add_history_entry(line.as_str()).unwrap();
                    match cmd.as_str() {
                        "cd" => {
                            let target = params.first().map_or_else(
                                || dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
                                PathBuf::from,
                            );
                            if let Err(e) = env::set_current_dir(&target) {
                                eprintln!("cd: {}: {}", target.display(), e);
                                last_success = false;
                                exit_code = 1;
                            } else {
                                last_success = true;
                                exit_code = 0;
                            }
                            continue;
                        }
                        "export" => {
                            last_success = true;
                            exit_code = 0;
                            for param in params {
                                if let Some((key, value)) = param.split_once('=') {
                                    env::set_var(key, value);
                                } else {
                                    eprintln!("export: invalid format: {}", param);
                                    last_success = false;
                                    exit_code = 1;
                                    continue;
                                }
                            }
                            continue;
                        }
                        "history" => {
                            for (idx, entry) in rl.history().iter().enumerate() {
                                println!("  {}  {}", idx + 1, entry);
                            }
                            last_success = true;
                            exit_code = 0;
                            continue;
                        }
                        "help" => {
                            println!("內建指令:");
                            println!("  cd [dir]       - 切換目錄");
                            println!("  export VAR=VAL - 設定環境變數");
                            println!("  history        - 顯示命令歷史");
                            println!("  help           - 顯示此幫助訊息");
                            println!("  exit           - 退出 Shell");
                            last_success = true;
                            exit_code = 0;
                            continue;
                        }
                        "exit" => {
                            println!("再見！");
                            break;
                        }
                        _ => {
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
