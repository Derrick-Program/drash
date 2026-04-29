use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

pub fn start_repl() -> rustyline::Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                println!("You entered: {}", line);
                if line.trim() == "exit" {
                    break;
                }
                rl.add_history_entry(line).unwrap();
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
    Ok(())
}
