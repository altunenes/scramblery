//! Recipes: scramble settings saved as JSON. Settings files written next to outputs can be
//! loaded as recipes too.

use slint::ComponentHandle;

use crate::{options, AppWindow, BatchState};

/// Reads scramble options from a recipe or any Scramblery settings file.
fn read(path: &std::path::Path) -> anyhow::Result<engine::ScrambleOptions> {
    let text = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let options = value.get("scramble_options").cloned().unwrap_or(value);
    Ok(serde_json::from_value(options)?)
}

fn load(ui: &AppWindow) -> anyhow::Result<()> {
    let Some(path) = rfd::FileDialog::new().set_title("Load recipe").add_filter("Recipe", &["json"]).pick_file() else {
        return Ok(());
    };
    let options = read(&path)?;
    options::apply_options(ui, &options);
    // Batch settings, when the file has them.
    if let Ok(settings) = serde_json::from_str::<engine::RunSettings>(&std::fs::read_to_string(&path)?) {
        let batch = ui.global::<BatchState>();
        batch.set_exemplars(settings.exemplars as f32);
        batch.set_series(settings.levels.is_some());
        if let Some(levels) = settings.levels {
            let text: Vec<String> = levels.iter().map(|l| l.to_string()).collect();
            batch.set_levels_text(text.join(", ").into());
        }
    }
    Ok(())
}

fn save(ui: &AppWindow) -> anyhow::Result<()> {
    let Some(mut path) = rfd::FileDialog::new()
        .set_title("Save recipe")
        .set_file_name("scramblery_recipe.json")
        .add_filter("Recipe", &["json"])
        .save_file()
    else {
        return Ok(());
    };
    path.set_extension("json");
    let batch = ui.global::<BatchState>();
    let settings = engine::RunSettings {
        exemplars: batch.get_exemplars().round().max(1.0) as usize,
        levels: crate::batch_page::levels(ui).ok().flatten(),
        ..engine::RunSettings::new(&options::scramble_options(ui))
    };
    engine::io::write_json(&settings, &path)
}

pub fn setup(ui: &AppWindow, report_error: impl Fn(&AppWindow, String) + Clone + 'static) {
    let params = ui.global::<crate::Params>();

    let weak = ui.as_weak();
    let report = report_error.clone();
    params.on_load_recipe(move || {
        let ui = weak.unwrap();
        if let Err(e) = load(&ui) {
            report(&ui, format!("Could not load recipe: {e:#}"));
        }
    });

    let weak = ui.as_weak();
    params.on_save_recipe(move || {
        let ui = weak.unwrap();
        if let Err(e) = save(&ui) {
            report_error(&ui, format!("Could not save recipe: {e:#}"));
        }
    });
}
