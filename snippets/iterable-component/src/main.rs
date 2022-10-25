use iced::widget::{column, container};
use iced::{executor, Application, Command, Element, Length, Settings, Theme};

use self::thing::Thing;

fn main() {
    App::run(Settings::default()).unwrap();
}

#[derive(Debug, Clone)]
enum Message {
    Thing(usize, thing::Message),
}

struct App {
    things: Vec<Thing>,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            App {
                things: vec![Thing::default(); 10],
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Example".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Thing(index, message) => {
                if let Some(thing) = self.things.get_mut(index) {
                    if let Some(event) = thing.update(message) {
                        match event {
                            thing::Event::Delete => {
                                self.things.remove(index);
                            }
                        }
                    }
                }

                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let things = column(
            self.things
                .iter()
                .enumerate()
                .map(|(index, thing)| {
                    thing
                        .view(index)
                        .map(move |message| Message::Thing(index, message))
                })
                .collect(),
        )
        .spacing(5);

        container(things)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

mod thing {
    use iced::widget::{button, container, row, text};
    use iced::{theme, Alignment, Element, Length};

    #[derive(Debug, Clone)]
    pub enum Event {
        Delete,
    }

    #[derive(Debug, Clone)]
    pub enum Message {
        Increment,
        Decrement,
        Delete,
    }

    #[derive(Debug, Default, Clone)]
    pub struct Thing {
        count: u64,
    }

    impl Thing {
        pub fn update(&mut self, message: Message) -> Option<Event> {
            match message {
                Message::Increment => {
                    self.count += 1;
                    None
                }
                Message::Decrement => {
                    self.count = self.count.saturating_sub(1);
                    None
                }
                Message::Delete => Some(Event::Delete),
            }
        }

        pub fn view(&self, index: usize) -> Element<Message> {
            let count = self.count;

            container(
                row![
                    text(format!("Thing {index} - Count: {count}")).width(Length::Fill),
                    button(text("Increment")).on_press(Message::Increment),
                    button(text("Decrement")).on_press(Message::Decrement),
                    button(text("Delete"))
                        .on_press(Message::Delete)
                        .style(theme::Button::Destructive)
                ]
                .spacing(5)
                .align_items(Alignment::Center),
            )
            .max_width(400)
            .into()
        }
    }
}
