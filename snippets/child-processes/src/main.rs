use iced::widget::{column, container};
use iced::{
    executor, subscription, window, Application, Command, Element, Event, Length, Settings,
    Subscription, Theme,
};

use self::backend::Backend;
use self::process::Process;

fn main() {
    App::run(Settings {
        exit_on_close_request: false,
        ..Default::default()
    })
    .unwrap();
}

#[derive(Debug)]
enum Message {
    Event(Event),
    Process(process::Message),
    Backend(backend::Message),
}

enum App {
    Idle,
    Running { backend: Backend, process: Process },
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (App::Idle, Command::none())
    }

    fn title(&self) -> String {
        "Example".into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            subscription::events().map(Message::Event),
            backend::run().map(Message::Backend),
        ])
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Event(Event::Window(window::Event::CloseRequested)) => {
                if let Self::Running { backend, .. } = self {
                    backend.close();
                }

                Command::none()
            }
            Message::Event(_) => Command::none(),
            Message::Process(message) => {
                if let Self::Running { backend, process } = self {
                    process.update(message, backend);
                }

                Command::none()
            }
            Message::Backend(message) => match message {
                backend::Message::Setup(backend) => {
                    *self = Self::Running {
                        backend,
                        process: Process::default(),
                    };

                    Command::none()
                }
                backend::Message::ProcessExited(_id, exited) => {
                    if let Self::Running { process, .. } = self {
                        process.exited(exited);
                    }

                    Command::none()
                }
                backend::Message::Closed => window::close(),
            },
        }
    }

    fn view(&self) -> Element<Message> {
        match self {
            App::Idle => column![].into(),
            App::Running { process, .. } => container(process.view().map(Message::Process))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(30)
                .center_x()
                .into(),
        }
    }
}

mod process {
    use std::io;

    use iced::widget::{button, column, container, row, scrollable, text, text_input};
    use iced::{Alignment, Element, Length};

    use crate::backend::{Backend, Exited};

    #[derive(Debug, Clone)]
    pub enum Message {
        Input(String),
        Run,
        Reset,
    }

    #[derive(Debug)]
    pub enum Process {
        Idle(String),
        Running(u32, String),
        Exited(String, Exited),
        Error(String, String),
    }

    impl Default for Process {
        fn default() -> Self {
            Self::Idle(String::new())
        }
    }

    impl Process {
        pub fn exited(&mut self, result: io::Result<Exited>) {
            if let Self::Running(_, command) = self {
                let command = std::mem::take(command);

                match result {
                    Ok(exited) => *self = Self::Exited(command, exited),
                    Err(err) => *self = Self::Error(command, err.to_string()),
                }
            }
        }

        pub fn update(&mut self, message: Message, backend: &Backend) {
            match message {
                Message::Input(input) => {
                    if let Self::Idle(command) = self {
                        *command = input;
                    }
                }
                Message::Run => {
                    if let Self::Idle(command) = self {
                        match backend.spawn(command) {
                            Ok(Some(id)) => {
                                *self = Self::Running(id, command.to_string());
                            }
                            Ok(None) => {
                                *self =
                                    Self::Error(std::mem::take(command), "Unknown Error".into());
                            }
                            Err(err) => {
                                *self = Self::Error(std::mem::take(command), err.to_string());
                            }
                        }
                    }
                }
                Message::Reset => {
                    *self = Self::default();
                }
            }
        }

        fn command(&self) -> &str {
            match self {
                Process::Idle(command) => command,
                Process::Running(_, command) => command,
                Process::Exited(command, _) => command,
                Process::Error(command, _) => command,
            }
        }

        fn active_input(&self) -> Element<Message> {
            row![
                container(
                    text_input("Command...", self.command(), Message::Input)
                        .on_submit(Message::Run)
                        .padding(5)
                )
                .width(Length::Fill)
                .max_width(400),
                button(text("Run")).on_press(Message::Run),
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .into()
        }

        fn inactive_input(&self) -> Element<Message> {
            row![
                container(text_input(self.command(), "", Message::Input).padding(5))
                    .width(Length::Fill)
                    .max_width(400),
                button(text("Run")),
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .into()
        }

        fn reset_input(&self) -> Element<Message> {
            row![
                container(text_input(self.command(), "", Message::Input).padding(5))
                    .width(Length::Fill)
                    .max_width(400),
                button(text("Reset")).on_press(Message::Reset),
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .into()
        }

        pub fn view(&self) -> Element<Message> {
            match self {
                Process::Idle(_) => self.active_input(),
                Process::Running(_, _) => self.inactive_input(),
                Process::Exited(_, exited) => {
                    let input = self.reset_input();

                    let status = text(format!("{}", exited.status));

                    let output = scrollable(column(
                        exited.stdout.lines().map(text).map(Element::from).collect(),
                    ));

                    column![input, status, output]
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .into()
                }
                Process::Error(_, error) => {
                    let input = self.reset_input();

                    column![input, text(format!("ERROR: {error}"))]
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .into()
                }
            }
        }
    }
}

mod backend {
    use std::process::{ExitStatus, Stdio};
    use std::time::Duration;

    use iced::futures::stream::FuturesUnordered;
    use iced::futures::{future, stream, FutureExt, StreamExt};
    use iced::{subscription, Subscription};
    use tokio::io::{AsyncReadExt, BufReader};
    use tokio::process::{Child, Command};
    use tokio::sync::mpsc::{self, Receiver, Sender};
    use tokio::{io, time};

    pub enum Event {
        Wait(u32, Child),
        Close,
    }

    #[derive(Debug)]
    pub enum Message {
        Setup(Backend),
        ProcessExited(u32, io::Result<Exited>),
        Closed,
    }

    pub enum Input {
        Event(Event),
        Process(u32, io::Result<ExitStatus>),
    }

    #[derive(Debug)]
    pub struct Backend {
        sender: Sender<Event>,
    }

    impl Backend {
        pub fn close(&self) {
            let _ = self.sender.blocking_send(Event::Close);
        }

        pub fn spawn(&self, command: &str) -> io::Result<Option<u32>> {
            let mut split = command.split(' ');

            let program = split.next().unwrap_or_default();
            let mut command = Command::new(program);
            command.args(split);
            command.stdout(Stdio::piped());

            let child = command.spawn()?;
            if let Some(id) = child.id() {
                let _ = self.sender.blocking_send(Event::Wait(id, child));

                return Ok(Some(id));
            }

            Ok(None)
        }
    }

    #[derive(Debug)]
    pub struct Exited {
        pub status: ExitStatus,
        pub stdout: String,
        pub stderr: String,
    }

    pub fn run() -> Subscription<Message> {
        enum State {
            Idle,
            Running {
                receiver: Receiver<Event>,
                processes: Vec<(u32, Child)>,
            },
            Closed,
        }

        subscription::unfold((), State::Idle, |state| async move {
            match state {
                State::Idle => {
                    let (sender, receiver) = mpsc::channel(5);

                    (
                        Some(Message::Setup(Backend { sender })),
                        State::Running {
                            receiver,
                            processes: vec![],
                        },
                    )
                }
                State::Running {
                    mut receiver,
                    mut processes,
                } => loop {
                    let input = {
                        let processes =
                            FuturesUnordered::from_iter(processes.iter_mut().map(|(id, child)| {
                                child.wait().map(|result| Input::Process(*id, result))
                            }));

                        let receiver = receiver
                            .recv()
                            .into_stream()
                            .filter_map(|event| async move { event.map(Input::Event) })
                            .boxed();

                        stream::select(processes, receiver)
                            .next()
                            .await
                            .expect("Await input")
                    };

                    match input {
                        Input::Event(event) => match event {
                            Event::Wait(id, child) => processes.push((id, child)),
                            Event::Close => {
                                if !processes.is_empty() {
                                    future::join_all(
                                        processes.iter_mut().map(|(_, child)| child.kill()),
                                    )
                                    .await;
                                }

                                return (Some(Message::Closed), State::Closed);
                            }
                        },
                        Input::Process(id, result) => {
                            if let Some(index) = processes.iter().position(|(i, _)| *i == id) {
                                let (_, mut process) = processes.remove(index);

                                let mut stdout = String::new();
                                if let Some(io) = process.stdout.as_mut() {
                                    let mut reader = BufReader::new(io);
                                    let _ = reader.read_to_string(&mut stdout).await;
                                }

                                let mut stderr = String::new();
                                if let Some(io) = process.stdout.as_mut() {
                                    let mut reader = BufReader::new(io);
                                    let _ = reader.read_to_string(&mut stderr).await;
                                }

                                let exited = result.map(|status| Exited {
                                    status,
                                    stdout,
                                    stderr,
                                });

                                return (
                                    Some(Message::ProcessExited(id, exited)),
                                    State::Running {
                                        receiver,
                                        processes,
                                    },
                                );
                            }
                        }
                    }
                },
                State::Closed => {
                    time::sleep(Duration::from_secs(1)).await;

                    (None, State::Closed)
                }
            }
        })
    }
}
