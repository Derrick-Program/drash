mod repl;
fn main() -> anyhow::Result<()> {
    repl::start_repl()?;
    Ok(())
}
