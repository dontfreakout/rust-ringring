fn main() {
    if let Err(_) = run() {
        // Silent failure — hooks must never block Claude Code
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
