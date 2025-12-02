use iced::widget::{button, column, combo_box, container, row, scrollable, text, text_input, Column, Text};
use iced::{executor, Alignment, Application, Command, Element, Length, Settings, Theme};
use std::process::Output;
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone)]
enum Tool {
    Hacker,
    Hli,
    Hackerc,
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tool::Hacker => write!(f, "hacker"),
            Tool::Hli => write!(f, "hli"),
            Tool::Hackerc => write!(f, "hackerc"),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SelectTool(String),
    UpdateArgs(String),
    RunCommand,
    CommandFinished(Result<Output, anyhow::Error>),
    SelectCommand(String),
}

struct HackerGui {
    selected_tool: Tool,
    tool_combo: combo_box::State<String>,
    command_combo: combo_box::State<String>,
    selected_command: Option<String>,
    args: String,
    output: String,
    running: bool,
}

impl Application for HackerGui {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let tools = vec![
            Tool::Hacker.to_string(),
            Tool::Hli.to_string(),
            Tool::Hackerc.to_string(),
        ];
        let mut app = HackerGui {
            selected_tool: Tool::Hacker,
            tool_combo: combo_box::State::new(tools),
            command_combo: combo_box::State::new(vec![]),
            selected_command: None,
            args: String::new(),
            output: String::new(),
            running: false,
        };
        app.update_commands();
        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("HackerOS GUI - CLI Tools Wrapper")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SelectTool(tool_str) => {
                self.selected_tool = match tool_str.as_str() {
                    "hacker" => Tool::Hacker,
                    "hli" => Tool::Hli,
                    "hackerc" => Tool::Hackerc,
                    _ => Tool::Hacker,
                };
                self.selected_command = None;
                self.update_commands();
                Command::none()
            }
            Message::SelectCommand(cmd) => {
                self.selected_command = Some(cmd);
                Command::none()
            }
            Message::UpdateArgs(args) => {
                self.args = args;
                Command::none()
            }
            Message::RunCommand => {
                if self.running {
                    return Command::none();
                }
                self.running = true;
                self.output = "Running command...".to_string();
                let tool = self.selected_tool.to_string();
                let cmd = self.selected_command.clone().unwrap_or_default();
                let args = self.args.clone();
                let full_args: Vec<String> = if args.is_empty() {
                    vec![cmd]
                } else {
                    let mut vec = vec![cmd];
                    vec.extend(args.split_whitespace().map(|s| s.to_string()));
                    vec
                };
                Command::perform(
                    async move {
                        let output = TokioCommand::new(tool)
                            .args(full_args)
                            .output()
                            .await
                            .map_err(anyhow::Error::from);
                        output
                    },
                    Message::CommandFinished,
                )
            }
            Message::CommandFinished(result) => {
                self.running = false;
                match result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        self.output = format!(
                            "Status: {}\n\nStdout:\n{}\n\nStderr:\n{}",
                            output.status,
                            stdout,
                            stderr
                        );
                    }
                    Err(err) => {
                        self.output = format!("Error: {}", err);
                    }
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let tool_selector = combo_box(
            &self.tool_combo,
            "Select Tool",
            Some(&self.selected_tool.to_string()),
            Message::SelectTool,
        );

        let command_selector = combo_box(
            &self.command_combo,
            "Select Command",
            self.selected_command.as_ref(),
            Message::SelectCommand,
        )
        .width(Length::Fixed(200.0));

        let args_input = text_input(
            "Enter additional arguments",
            &self.args,
        )
        .on_input(Message::UpdateArgs)
        .padding(10)
        .width(Length::Fill);

        let run_button = button(if self.running { "Running..." } else { "Run Command" })
            .on_press(Message::RunCommand)
            .padding(10);

        let output_text = text(&self.output).size(14);

        let content = column![
            row![
                tool_selector.width(Length::Fixed(200.0)),
                command_selector,
                args_input,
                run_button,
            ]
            .spacing(10)
            .align_items(Alignment::Center),
            container(scrollable(output_text))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
                .style(iced::theme::Container::Box),
        ]
        .spacing(20)
        .padding(20)
        .align_items(Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl HackerGui {
    fn update_commands(&mut self) {
        let commands = match self.selected_tool {
            Tool::Hacker => vec![
                "unpack".to_string(),
                "help-ui".to_string(),
                "docs".to_string(),
                "install".to_string(),
                "remove".to_string(),
                "flatpak-install".to_string(),
                "flatpak-remove".to_string(),
                "system".to_string(),
                "run".to_string(),
                "update".to_string(),
                "game".to_string(),
                "hacker-lang".to_string(),
                "ascii".to_string(),
                "shell".to_string(),
                "enter".to_string(),
                "remove-container".to_string(),
                "restart".to_string(),
                "plugin".to_string(),
                "enable".to_string(),
                "disable".to_string(),
                "help".to_string(),
            ],
            Tool::Hli => vec![
                "run".to_string(),
                "compile".to_string(),
                "check".to_string(),
                "init".to_string(),
                "clean".to_string(),
                "repl".to_string(),
                "editor".to_string(),
                "unpack".to_string(),
                "docs".to_string(),
                "tutorials".to_string(),
                "version".to_string(),
                "help".to_string(),
                "syntax".to_string(),
                "help-ui".to_string(),
            ],
            Tool::Hackerc => vec![
                "run".to_string(),
                "compile".to_string(),
                "help".to_string(),
            ],
        };
        self.command_combo = combo_box::State::new(commands);
    }
}

fn main() -> iced::Result {
    HackerGui::run(Settings::default())
}
