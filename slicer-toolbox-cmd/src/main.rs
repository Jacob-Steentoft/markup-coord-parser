use anyhow::{anyhow, Context, Result};
use compress_tools::ArchiveContents;
use csv::Writer;
use icu_collator::{AlternateHandling, Collator, CollatorOptions, Numeric};
use merge_whitespace_utils::merge_whitespace;
use rfd::FileDialog;
use slicer_toolbox_core::SlicerMarkup;
use std::collections::HashMap;
use std::fs::File;
use std::ops::Neg;
use std::path::Path;
use walkdir::WalkDir;

struct Coord {
    r: f64,
    a: f64,
    s: f64,
}

fn main() -> Result<()> {
    let Some(path) = FileDialog::new()
        .set_title("Select folder to import from")
        .pick_folder()
    else {
        return Err(anyhow!("No directory selected"));
    };

    let mut all_file_coords = Vec::new();
    for entry_result in WalkDir::new(&path).into_iter().filter(|x| {
        x.as_ref()
            .is_ok_and(|x1| x1.file_name().to_str().is_some_and(|t| t.ends_with(".mrb")))
    }) {
        let entry = entry_result?;
        let file_name = entry
            .file_name()
            .to_str()
            .context("Failed to read the name of the file")?
            .to_string();

        let file =
            File::open(entry.path()).context(format!("Could not open file: {}", file_name))?;

        let mut all_coords = HashMap::new();
        let mut buffer = Vec::new();
        for context in compress_tools::ArchiveIteratorBuilder::new(&file)
            .filter(|name, _| name.ends_with(".mrk.json"))
            .build()?
        {
            match context {
                ArchiveContents::StartOfEntry(_, _) => {
                    buffer.clear();
                    continue;
                }
                ArchiveContents::DataChunk(buf) => {
                    buffer.extend(&buf);
                    continue;
                }
                ArchiveContents::EndOfEntry => {}
                ArchiveContents::Err(err) => {
                    return Err(anyhow!("Failed to parse archive data with err: {0}", err))
                }
            }

            let markup_content: SlicerMarkup =
                serde_json::from_slice(&buffer).context("Failed to parse a json file")?;

            for markup in markup_content.markups {
                let coords = markup.coordinate_system;
                if coords.len() != 3 {
                    return Err(anyhow!("Invalid coordinate system. Should be 3 characters"));
                }

                let chars = coords.chars().collect::<Vec<_>>();
                for control_point in markup.control_points {
                    let r = convert_to_ras(chars[0], &'r', &'l', control_point.position[0])?;
                    let a = convert_to_ras(chars[1], &'a', &'p', control_point.position[1])?;
                    let s = convert_to_ras(chars[2], &'s', &'i', control_point.position[2])?;

                    let label = merge_whitespace(control_point.label.trim()).to_string();

                    all_coords.insert(label, Coord { r, a, s });
                }
            }
        }

        all_file_coords.push((file_name, all_coords));
    }

    // Get unique landmarks
    let mut landmarks = Vec::new();
    all_file_coords.iter().for_each(|(_, data)| {
        data.iter().for_each(|(landmark, _)| {
            if !landmarks.contains(landmark) {
                landmarks.push(landmark.clone())
            }
        })
    });

    // Sort and cleanup
    let mut options = CollatorOptions::default();
    options.numeric = Some(Numeric::On);
    options.alternate_handling = Some(AlternateHandling::Shifted);
    let collator = Collator::try_new(Default::default(), options).map_err(|e| anyhow!(e))?;

    all_file_coords.sort_by(|(lhs, _), (rhs, _)| collator.compare(lhs, rhs));
    landmarks.sort_by(|lhs, rhs| collator.compare(lhs, rhs));

    // Data creation
    create_main_data(&path, &all_file_coords, &landmarks)?;
    create_statistics(&path, &all_file_coords, &landmarks)?;

    dont_disappear::any_key_to_continue::default();
    Ok(())
}

fn create_statistics(
    path: &Path,
    all_file_coords: &[(String, HashMap<String, Coord>)],
    landmarks: &[String],
) -> Result<()> {
    let path = path.join("statistics.csv");
    let mut writer = Writer::from_writer(File::create(&path)?);

    writer.write_field("Samples")?;
    for landmark in landmarks {
        writer.write_field(format!("{}__A", landmark))?;
        writer.write_field(format!("{}__S", landmark))?;
    }

    new_line(&mut writer)?;

    for (name, coord_per_land) in all_file_coords {
        writer.write_field(name)?;
        for landmark in landmarks {
            match coord_per_land.get(landmark) {
                None => {
                    write_filler_lines(&mut writer, 2)?;
                }
                Some(coord) => {
                    writer.write_field(coord.a.to_string())?;
                    writer.write_field(coord.s.to_string())?;
                }
            }
        }
        new_line(&mut writer)?;
    }

    println!("Created statistics file at: {}", path.to_str().unwrap());
    Ok(())
}

fn create_main_data(
    path: &Path,
    all_file_coords: &[(String, HashMap<String, Coord>)],
    landmarks: &[String],
) -> Result<()> {
    let path = path.join("main data.csv");
    let mut writer = Writer::from_writer(File::create(&path)?);

    for (name, _) in all_file_coords {
        writer.write_field(name)?;
        writer.write_field("R")?;
        writer.write_field("A")?;
        writer.write_field("S")?;
    }
    new_line(&mut writer)?;

    for landmark in landmarks {
        for (_, coord_per_land) in all_file_coords {
            writer.write_field(landmark)?;
            match coord_per_land.get(landmark) {
                None => {
                    write_filler_lines(&mut writer, 3)?;
                }
                Some(coord) => {
                    writer.write_field(coord.r.to_string())?;
                    writer.write_field(coord.a.to_string())?;
                    writer.write_field(coord.s.to_string())?;
                }
            }
        }

        new_line(&mut writer)?;
    }

    println!("Created main data file at: {}", path.to_str().unwrap());
    Ok(())
}

fn convert_to_ras(actual: char, positive: &char, negative: &char, position: f64) -> Result<f64> {
    Ok(if actual.eq_ignore_ascii_case(positive) {
        position
    } else if actual.eq_ignore_ascii_case(negative) {
        position.neg()
    } else {
        return Err(anyhow!(
            "Invalid coordinate named: {}. Should be either {} or {}",
            actual,
            positive,
            negative
        ));
    })
}

fn write_filler_lines(writer: &mut Writer<File>, count: usize) -> Result<()> {
    for _ in 0..count {
        writer.write_field("")?;
    }
    Ok(())
}

fn new_line(writer: &mut Writer<File>) -> Result<()> {
    writer.write_record(None::<&[u8]>)?;
    Ok(())
}
