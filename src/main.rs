mod app;
mod rules;
mod ui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let app = app::App::new();
    ui::run(app)
}
