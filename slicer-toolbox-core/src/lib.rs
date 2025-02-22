use anyhow::{Context, Result, anyhow};
use compress_tools::{ArchiveContents, ArchiveIterator};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::{Read, Seek};
use std::marker::PhantomData;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct SlicerMarkup {
	pub markups: Vec<Markups>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Markups {
	pub coordinate_system: String,
	pub control_points: Vec<ControlPoint>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct ControlPoint {
	pub label: String,
	pub position: [f64; 3],
}

#[derive(Debug)]
pub struct Coordinates {
	pub coord_1: String,
	pub coord_2: String,
	pub coord_3: String,
	pub coordinates: Vec<Coordinate>,
}

#[derive(Debug)]
pub struct Coordinate {
	pub name: String,
	pub x: f64,
	pub y: f64,
	pub z: f64,
}

pub fn parse_slicer_markups(dir: impl AsRef<Path>) -> Result<Coordinates> {
	let mut coordinate_system = None;
	let mut coordinates = Vec::new();
	for entry_result in WalkDir::new(dir).into_iter().filter(|x| {
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
				coordinate_system = Some(coords)
			} else if coordinate_system != Some(coords) {
				return Err(anyhow!(
					"Multiple different coordinate systems found. Please make sure to only export using one type"
				));
			}

			for control_point in markup.control_points {
				coordinates.push(Coordinate {
					name: control_point.label,
					x: control_point.position[0],
					y: control_point.position[1],
					z: control_point.position[2],
				})
			}
		}
	}

	if let Some(coordinate_system) = coordinate_system {
		Ok(Coordinates {
			coord_1: coordinate_system[0..1].to_string(),
			coord_2: coordinate_system[1..2].to_string(),
			coord_3: coordinate_system[2..3].to_string(),
			coordinates,
		})
	} else {
		Err(anyhow!("No coordinate system found"))
	}
}

pub struct ArchiveFileIterator<R: Read + Seek, T: DeserializeOwned> {
	buffer: Vec<u8>,
	archive_iter: ArchiveIterator<R>,
	phantom_data: PhantomData<T>,
}

impl<R: Read + Seek, T: DeserializeOwned> ArchiveFileIterator<R, T> {
	fn new(iter: ArchiveIterator<R>) -> Self {
		Self {
			archive_iter: iter,
			buffer: Vec::new(),
			phantom_data: PhantomData::default(),
		}
	}
}

impl<R: Read + Seek, T: DeserializeOwned> Iterator for ArchiveFileIterator<R, T> {
	type Item = Result<T>;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let Some(context) = self.archive_iter.next() else {
				return None;
			};

			match context {
				ArchiveContents::StartOfEntry(_, _) => {
					self.buffer.clear();
					continue;
				}
				ArchiveContents::DataChunk(buf) => {
					self.buffer.extend(&buf);
					continue;
				}
				ArchiveContents::EndOfEntry => {
					return Some(
						serde_json::from_slice::<T>(&self.buffer)
							.context("Failed to parse a json file"),
					);
				}
				ArchiveContents::Err(err) => {
					return Some(Err(anyhow!(
						"Failed to parse archive data with err: {0}",
						err
					)));
				}
			}
		}
	}
}

impl<R: Read + Seek> ArchiveCollector for ArchiveIterator<R> {
	fn into_de_iter<T: for<'de> Deserialize<'de>>(
		self,
	) -> ArchiveFileIterator<impl Read + Seek, T> {
		ArchiveFileIterator::new(self)
	}
}

pub trait ArchiveCollector {
	fn into_de_iter<T: for<'de> Deserialize<'de>>(
		self,
	) -> ArchiveFileIterator<impl Read + Seek, T>;
}
