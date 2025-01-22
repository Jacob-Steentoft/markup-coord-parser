use iced::widget::{button, column, container, row, scrollable, text, Column, Container};
use iced::{Alignment, Element, Length};
use rfd::FileDialog;
use slicer_toolbox_core::{parse_slicer_markups, Coordinate, Coordinates};

#[derive(Debug, Clone)]
enum Message {
    Open,
}

#[derive(Default)]
struct State {
    coordinates: Option<Coordinates>,
}

pub fn main() -> iced::Result {
    iced::run("Slicer Toolbox", update, view)
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::Open => {
            let Some(path) = FileDialog::new()
                .set_title("Select folder to import from")
                .pick_folder()
            else {
                return;
            };
            if let Ok(coordinates) = parse_slicer_markups(path) {
                state.coordinates = Some(coordinates)
            }
        }
    }
}

fn view(state: &State) -> Element<Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(button("Select folder").on_press(Message::Open));

    if let Some(coordinates) = &state.coordinates {
        column = column.push(coordinate_list_view(coordinates));
    };

    column.into()
}

fn coordinate_view(coordinate: &Coordinate) -> Element<'static, Message> {
    row!(
        text(coordinate.name.clone()),
        text(coordinate.x),
        text(coordinate.y),
        text(coordinate.z)
    )
    .align_y(Alignment::Center)
    .spacing(30)
    .into()
}

fn coordinate_list_view(coordinates: &Coordinates) -> Element<'static, Message> {
    let mut column = Column::new()
        .spacing(20)
        .align_x(Alignment::Center)
        .width(Length::Shrink);

    column = column.push(row![
        text("Name"),
        text(coordinates.coord_1.clone()),
        text(coordinates.coord_2.clone()),
        text(coordinates.coord_3.clone())
    ]);

    for coord in coordinates.coordinates.iter() {
        column = column.push(coordinate_view(coord));
    }

    scrollable(container(column))
        .height(250.0)
        .width(900)
        .into()
}
