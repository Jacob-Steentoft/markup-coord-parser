use anyhow::{anyhow, Result};
use csv::Writer;
use rfd::FileDialog;
use slicer_toolbox_core::SlicerMarkup;
use std::fs::File;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let Some(path) = FileDialog::new()
        .set_title("Select folder to import from")
        .pick_folder()
    else {
        return Err(anyhow!("No directory selected"));
    };
    let csv_path = path.join("coords.csv");
    let mut csv_writer = Writer::from_writer(File::create(&csv_path)?);
    let mut count = 0;
    let mut coordinate_system = None;
    for entry_result in WalkDir::new(path).into_iter().filter(|x| {
        x.as_ref().is_ok_and(|x1| {
            x1.file_name()
                .to_str()
                .is_some_and(|t| t.ends_with(".mrk.json"))
        })
    }) {
        let entry = entry_result?;
        let markup_content: SlicerMarkup = serde_json::from_reader(File::open(entry.path())?)?;
        for markup in markup_content.markups {
            let coords = markup.coordinate_system;
            if coords.len() != 3 {
                return Err(anyhow!("Invalid coordinate system. Should be 3 characters"));
            }

            if coordinate_system.is_none() {
                csv_writer.write_record(["Name", &coords[0..1], &coords[1..2], &coords[2..3]])?;
                coordinate_system = Some(coords).clone()
            } else if coordinate_system != Some(coords) {
                return Err(anyhow!("Multiple different coordinate systems found. Please make sure to only export using one type"));
            }

            for control_point in markup.control_points {
                csv_writer.write_record(&[
                    control_point.label,
                    control_point.position[0].to_string(),
                    control_point.position[1].to_string(),
                    control_point.position[2].to_string(),
                ])?;
                count += 1;
            }
        }
    }
    println!(
        "Saved {count} coordinates to \"{0}\"",
        csv_path.to_string_lossy()
    );
    dont_disappear::enter_to_continue::default();
    Ok(())
}
