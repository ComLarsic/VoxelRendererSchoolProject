use voxeltracer::app::App;

fn main() -> anyhow::Result<()> {
    // Create and run the app
    let mut app = App::new()?;
    app.run()?;
    Ok(())
}
