use iced::widget::{column, container, text};
use iced::{executor, window, Alignment, Application, Command, Element, Length, Settings, Theme};

use self::hover::hover;

fn main() {
    App::run(Settings {
        window: window::Settings {
            size: (350, 350),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
    .unwrap();
}

struct App;

impl Application for App {
    type Executor = executor::Default;
    type Message = ();
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<()>) {
        (App, Command::none())
    }

    fn title(&self) -> String {
        "Hover Component".into()
    }

    fn update(&mut self, _message: ()) -> Command<()> {
        Command::none()
    }

    fn view(&self) -> Element<()> {
        let title = text("Hover the face").size(24);
        let face = hover(|hovered| text(if hovered { "😎" } else { "🙂" }).size(96).into());

        container(
            column![title, face]
                .align_items(Alignment::Center)
                .spacing(8),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    }
}

mod hover {
    use iced::Element;

    pub fn hover<'a, Message: 'a>(
        f: impl Fn(bool) -> Element<'a, Message> + 'a,
    ) -> Element<'a, Message> {
        component::Hover::new(f).into()
    }

    mod component {
        use iced::widget::{component, Component};
        use iced::{Element, Renderer};

        pub enum Event<M> {
            Change(super::widget::Cursor),
            Message(M),
        }

        pub struct Hover<'a, Message> {
            view: Box<dyn Fn(bool) -> Element<'a, Message> + 'a>,
        }

        impl<'a, Message> Hover<'a, Message> {
            pub fn new(view: impl Fn(bool) -> Element<'a, Message> + 'a) -> Self {
                Self {
                    view: Box::new(view),
                }
            }
        }

        impl<'a, Message> Component<Message, Renderer> for Hover<'a, Message> {
            type State = bool;
            type Event = Event<Message>;

            fn update(&mut self, hovered: &mut Self::State, event: Self::Event) -> Option<Message> {
                match event {
                    Event::Change(change) => {
                        match change {
                            super::widget::Cursor::Entered => *hovered = true,
                            super::widget::Cursor::Left => *hovered = false,
                        }
                        None
                    }
                    Event::Message(message) => Some(message),
                }
            }

            fn view(&self, hovered: &Self::State) -> Element<'_, Self::Event> {
                super::widget::Hover::new((self.view)(*hovered).map(Event::Message), Event::Change)
                    .into()
            }
        }

        impl<'a, Message> From<Hover<'a, Message>> for Element<'a, Message>
        where
            Message: 'a,
        {
            fn from(hover: Hover<'a, Message>) -> Self {
                component(hover)
            }
        }
    }

    mod widget {
        use iced::advanced::widget::{self, tree};
        use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
        use iced::{Element, Renderer, Theme};

        pub enum Cursor {
            Entered,
            Left,
        }

        pub struct Hover<'a, Message> {
            content: Element<'a, Message>,
            on_change: Box<dyn Fn(Cursor) -> Message + 'a>,
        }

        impl<'a, Message> Hover<'a, Message> {
            pub fn new(
                content: impl Into<Element<'a, Message>>,
                on_change: impl Fn(Cursor) -> Message + 'a,
            ) -> Self {
                Self {
                    content: content.into(),
                    on_change: Box::new(on_change),
                }
            }
        }

        impl<'a, Message> Widget<Message, Renderer> for Hover<'a, Message> {
            fn width(&self) -> iced::Length {
                self.content.as_widget().width()
            }

            fn height(&self) -> iced::Length {
                self.content.as_widget().height()
            }

            fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
                self.content.as_widget().layout(renderer, limits)
            }

            fn tag(&self) -> widget::tree::Tag {
                struct Marker;
                tree::Tag::of::<Marker>()
            }

            fn state(&self) -> widget::tree::State {
                tree::State::new(false)
            }

            fn children(&self) -> Vec<widget::Tree> {
                vec![widget::Tree::new(&self.content)]
            }

            fn diff(&self, tree: &mut widget::Tree) {
                tree.diff_children(&[&self.content]);
            }

            fn draw(
                &self,
                tree: &widget::Tree,
                renderer: &mut Renderer,
                theme: &Theme,
                style: &renderer::Style,
                layout: Layout<'_>,
                cursor_position: iced::Point,
                viewport: &iced::Rectangle,
            ) {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor_position,
                    viewport,
                )
            }

            fn on_event(
                &mut self,
                tree: &mut widget::Tree,
                event: iced::Event,
                layout: Layout<'_>,
                cursor_position: iced::Point,
                renderer: &Renderer,
                clipboard: &mut dyn Clipboard,
                shell: &mut Shell<'_, Message>,
            ) -> iced::event::Status {
                let hovered = tree.state.downcast_mut::<bool>();
                let prev_hovered = *hovered;
                *hovered = layout.bounds().contains(cursor_position);

                match (prev_hovered, *hovered) {
                    (true, false) => {
                        shell.publish((self.on_change)(Cursor::Left));
                    }
                    (false, true) => {
                        shell.publish((self.on_change)(Cursor::Entered));
                    }
                    _ => {}
                }

                self.content.as_widget_mut().on_event(
                    &mut tree.children[0],
                    event,
                    layout,
                    cursor_position,
                    renderer,
                    clipboard,
                    shell,
                )
            }

            fn mouse_interaction(
                &self,
                tree: &widget::Tree,
                layout: Layout<'_>,
                cursor_position: iced::Point,
                viewport: &iced::Rectangle,
                renderer: &Renderer,
            ) -> mouse::Interaction {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    layout,
                    cursor_position,
                    viewport,
                    renderer,
                )
            }

            fn operate(
                &self,
                tree: &mut widget::Tree,
                layout: Layout<'_>,
                renderer: &Renderer,
                operation: &mut dyn widget::Operation<Message>,
            ) {
                self.content
                    .as_widget()
                    .operate(&mut tree.children[0], layout, renderer, operation)
            }

            fn overlay<'b>(
                &'b mut self,
                tree: &'b mut widget::Tree,
                layout: Layout<'_>,
                renderer: &Renderer,
            ) -> Option<overlay::Element<'b, Message, Renderer>> {
                self.content
                    .as_widget_mut()
                    .overlay(&mut tree.children[0], layout, renderer)
            }
        }

        impl<'a, Message> From<Hover<'a, Message>> for Element<'a, Message>
        where
            Message: 'a,
        {
            fn from(hover: Hover<'a, Message>) -> Self {
                Element::new(hover)
            }
        }
    }
}
