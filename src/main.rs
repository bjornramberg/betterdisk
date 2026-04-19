mod scanner;
mod state;
mod ui;

fn main() {
    if let Err(e) = ui::run_app() {
        eprintln!("Error: {}", e);
    }
}