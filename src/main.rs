mod repl;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use signal_hook::{
    consts::{SIGCHLD, SIGHUP, SIGINT, SIGTERM},
    iterator::Signals,
};
use std::{process, thread};
fn main() -> anyhow::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGCHLD, SIGHUP, SIGTERM])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGHUP => {
                    println!("\n[系統] 偵測到 SIGHUP，正在通知子程序並清理...");
                    cleanup_and_exit();
                }
                SIGTERM => {
                    println!("\n[系統] 收到 SIGTERM，準備關閉 Shell...");
                    cleanup_and_exit();
                }
                SIGINT => {
                    println!("\n收到 SIGINT (Ctrl-C)，但我不退出！");
                }
                SIGCHLD => {
                    while let Ok(status) = waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                        if status == WaitStatus::StillAlive {
                            break;
                        }
                        if let Some(pid) = status.pid() {
                            println!("子进程 {} 已退出", pid);
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    });
    repl::start_repl()?;
    Ok(())
}

fn cleanup_and_exit() {
    // 這裡可以做幾件事：
    // a. 向所有還在運行的背景作業發送 SIGHUP (如果你的 Shell 有紀錄 PID)
    // b. 強制儲存 rustyline 的歷史紀錄到檔案
    // c. 釋放特定的系統資源

    println!("清理完畢。再見！");
    process::exit(0);
}
