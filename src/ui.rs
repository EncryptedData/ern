use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Terminal;

use crate::app::{App, Panel, RuleDialogMode, RuleInputStep};
use crate::rules::{apply_rules, CaseTransform, RenameRule};

pub fn run(mut app: App) -> color_eyre::Result<()> {
    let mut terminal = init_terminal()?;
    let result = main_loop(&mut app, &mut terminal);
    restore_terminal();
    result
}

fn init_terminal(
) -> color_eyre::Result<Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() {
    crossterm::terminal::disable_raw_mode().ok();
    execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen).ok();
    execute!(std::io::stdout(), crossterm::event::DisableMouseCapture).ok();
}

fn main_loop(
    app: &mut App,
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
) -> color_eyre::Result<()> {
    while app.running {
        let visible_height = terminal
            .size()
            .ok()
            .map(|s| s.height)
            .unwrap_or(24)
            .saturating_sub(3);

        terminal.draw(|frame| render(frame, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(app, key, visible_height);
            }
        }
    }
    Ok(())
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(frame.area());

    let panel_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
        .split(chunks[0]);

    render_rules_panel(frame, panel_chunks[0], app);
    render_files_panel(frame, panel_chunks[1], app);
    render_statusbar(frame, chunks[1], app);
}

fn render_rules_panel(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Rules;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(
            format!(" Rename Rules ({}) ", app.rules.len()),
            title_style,
        )))
        .border_style(border_style);

    let total_items = app.rules.len() + 1;
    let is_active = app.active_panel == Panel::Rules;

    let mut rows: Vec<Row> = app
        .rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let row_style = if i == app.rule_cursor {
                if is_active {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::Gray).bg(Color::DarkGray)
                }
            } else {
                Style::default()
            };
            Row::new(vec![format!("[{}] {}", i, rule)]).style(row_style)
        })
        .collect();

    let row_style = if total_items - 1 == app.rule_cursor {
        if is_active {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Gray).bg(Color::DarkGray)
        }
    } else {
        Style::default()
    };
    rows.push(Row::new(vec!["Add Rule"]).style(row_style));

    let table = Table::new(rows, [Constraint::Min(0)]).block(block);
    frame.render_widget(table, area);

    if app.dialog_mode != RuleDialogMode::None {
        render_dialog(frame, area, app);
    }
}

fn render_dialog(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    match app.dialog_mode {
        RuleDialogMode::SelectRule => render_rule_select_dialog(frame, area, app),
        RuleDialogMode::ConfirmQuit => render_confirm_quit_dialog(frame, area, app),
        _ => render_rule_input_dialog(frame, area, app),
    }
}

fn render_rule_select_dialog(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let items = [
        "Find/Replace",
        "Find/Replace (Regex)",
        "Add Prefix",
        "Add Suffix",
        "Change Case",
        "Remove Pattern",
        "Numbering",
    ];

    let item_count: u16 = items.len() as u16;
    let padding: u16 = 5;
    let dialog_height = item_count + padding;
    let dialog_width: u16 = 40;
    let x = area.x + area.width.saturating_sub(dialog_width) / 2;
    let y = area.y + area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect {
        x,
        y,
        width: dialog_width.min(area.width),
        height: dialog_height.min(area.height),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Line::from(Span::styled(
            " Add Rule ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::REVERSED),
        )));

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Select a rule type:",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
    ];

    for (i, item) in items.iter().enumerate() {
        let key = (b'1' + i as u8) as char;
        let style = if i == app.dialog_cursor {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!(
                "  {}  {}  {}",
                key,
                item,
                if i == app.dialog_cursor { ">>" } else { "  " }
            ),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  enter:select  esc:cancel",
        Style::default().fg(Color::Gray),
    )));

    let content = Paragraph::new(lines).block(block);
    frame.render_widget(content, dialog_area);
}

fn render_confirm_quit_dialog(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let items = ["Yes", "No"];
    let item_count: u16 = items.len() as u16;
    let padding: u16 = 5;
    let dialog_height = item_count + padding;
    let dialog_width: u16 = 30;
    let x = area.x + area.width.saturating_sub(dialog_width) / 2;
    let y = area.y + area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect {
        x,
        y,
        width: dialog_width.min(area.width),
        height: dialog_height.min(area.height),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(Line::from(Span::styled(
            " Quit? ",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::REVERSED),
        )));

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Are you sure?",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
    ];

    for (i, item) in items.iter().enumerate() {
        let key = (b'y' + i as u8) as char;
        let style = if i == app.dialog_cursor {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!(
                "  {}  {}  {}",
                key,
                item,
                if i == app.dialog_cursor { ">>" } else { "  " }
            ),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  enter:confirm  esc:no  arrows:navigate",
        Style::default().fg(Color::Gray),
    )));

    let content = Paragraph::new(lines).block(block);
    frame.render_widget(content, dialog_area);
}

fn render_rule_input_dialog(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let dialog_height: u16 = 14;
    let dialog_width: u16 = 50;
    let x = area.x + area.width.saturating_sub(dialog_width) / 2;
    let y = area.y + area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect {
        x,
        y,
        width: dialog_width.min(area.width),
        height: dialog_height.min(area.height),
    };

    let title = match app.dialog_mode {
        RuleDialogMode::FindReplace => " Find/Replace Rule ",
        RuleDialogMode::FindReplaceRegex => " Find/Replace (Regex) Rule ",
        RuleDialogMode::Prefix => " Add Prefix Rule ",
        RuleDialogMode::Suffix => " Add Suffix Rule ",
        RuleDialogMode::Case => " Change Case Rule ",
        RuleDialogMode::RemovePattern => " Remove Pattern Rule ",
        RuleDialogMode::Numbering => " Numbering Rule ",
        _ => " Add Rule ",
    };

    let prompt = match (app.dialog_mode, app.rule_input_step) {
        (RuleDialogMode::FindReplace, RuleInputStep::InputText) => "Find string:",
        (RuleDialogMode::FindReplace, RuleInputStep::InputReplace) => "Replace with:",
        (RuleDialogMode::FindReplaceRegex, RuleInputStep::InputText) => "Regex pattern:",
        (RuleDialogMode::FindReplaceRegex, RuleInputStep::InputReplace) => "Replace with:",
        (RuleDialogMode::Prefix, RuleInputStep::InputText) => "Prefix string:",
        (RuleDialogMode::Suffix, RuleInputStep::InputText) => "Suffix string:",
        (RuleDialogMode::RemovePattern, RuleInputStep::InputText) => "Pattern to remove:",
        (RuleDialogMode::Numbering, RuleInputStep::InputNumber) => "Start number:",
        (RuleDialogMode::Numbering, RuleInputStep::InputWidth) => "Width (digits):",
        (RuleDialogMode::Numbering, RuleInputStep::InputPlaceholder) => "Placeholder (e.g. ##):",
        (RuleDialogMode::Case, RuleInputStep::SelectCase) => {
            "Case: 1=UPPER 2=lower 3=Title 4=tOGGLE"
        }
        _ => "Input:",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::REVERSED),
        )));

    let input_line = Line::from(Span::styled(
        format!("> {} {}|", prompt, app.rule_input_buffer),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));

    let hint_line = match app.dialog_mode {
        RuleDialogMode::Case => Line::from(Span::styled(
            "  enter:confirm  esc:cancel",
            Style::default().fg(Color::Gray),
        )),
        RuleDialogMode::Suffix => Line::from(Span::styled(
            "  enter:next  space:toggle checkbox  esc:cancel",
            Style::default().fg(Color::Gray),
        )),
        _ => Line::from(Span::styled(
            "  enter:next  esc:cancel",
            Style::default().fg(Color::Gray),
        )),
    };

    #[allow(clippy::useless_vec)]
    let lines = {
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Enter the value:",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            input_line,
            Line::from(""),
            hint_line,
        ];

        if app.dialog_mode == RuleDialogMode::Suffix {
            let cb_text = if app.suffix_after_extension {
                "[x] After extension"
            } else {
                "[ ] After extension"
            };
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                cb_text,
                Style::default().fg(Color::Yellow),
            )));
        }

        lines
    };

    let content = Paragraph::new(lines).block(block);
    frame.render_widget(content, dialog_area);
}

fn render_files_panel(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Files;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(
            format!(" Files ({}) ", app.current_dir.display()),
            title_style,
        )))
        .border_style(border_style);

    let visible_height = (area.height as usize).saturating_sub(2);
    let total_files = app.files.len();

    let start = app.file_scroll;
    let end = (start + visible_height / 2).min(total_files);

    let mut rows: Vec<Row<'_>> = Vec::new();

    for i in start..end {
        let file = &app.files[i];
        let new_name = apply_rules(file, &app.rules, i as u32);

        let file_style = if i == app.file_cursor {
            if is_active {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::Gray).bg(Color::DarkGray)
            }
        } else {
            Style::default()
        };

        let is_selected = i == app.file_cursor;
        let output_prefix_style = if is_selected {
            if is_active {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::Gray).bg(Color::DarkGray)
            }
        } else {
            Style::default()
        };
        let output_style = if new_name != *file {
            if is_selected {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Red)
            }
        } else {
            Style::default()
        };

        rows.push(Row::new(vec![format!("File: {}", file)]).style(file_style));
        if new_name != *file {
            let output_bg = if is_selected {
                if is_active {
                    Color::White
                } else {
                    Color::DarkGray
                }
            } else {
                Color::Reset
            };
            let output_line = Line::from(vec![
                Span::styled("Output: ", output_prefix_style),
                Span::styled(new_name.clone(), output_style),
            ]).patch_style(Style::default().bg(output_bg));
            rows.push(Row::new(vec![output_line]));
        } else {
            rows.push(Row::new(vec![format!("Output: {}", new_name)]).style(output_prefix_style));
        }
    }

    let table = Table::new(rows, [Constraint::Min(0)]).block(block);
    frame.render_widget(table, area);
}

fn render_statusbar(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let help_text = if app.dialog_mode != RuleDialogMode::None {
        match app.dialog_mode {
            RuleDialogMode::ConfirmQuit => "  << Quit Confirmation >>  enter:confirm  esc:no",
            RuleDialogMode::SelectRule => "  << Add Rule >>  enter:select  1-6:select  esc:cancel",
            RuleDialogMode::FindReplace => match app.rule_input_step {
                RuleInputStep::InputText => "  Find string: enter:next  esc:cancel",
                RuleInputStep::InputReplace => "  Replace with: enter:confirm  esc:cancel",
                _ => "  << Find/Replace >>  enter:confirm  esc:cancel",
            },
            RuleDialogMode::FindReplaceRegex => match app.rule_input_step {
                RuleInputStep::InputText => "  Regex pattern: enter:next  esc:cancel",
                RuleInputStep::InputReplace => "  Replace with: enter:confirm  esc:cancel",
                _ => "  << Find/Replace (Regex) >>  enter:confirm  esc:cancel",
            },
            RuleDialogMode::Prefix => "  << Add Prefix >>  enter:confirm  esc:cancel",
            RuleDialogMode::Suffix => "  << Add Suffix >>  enter:confirm  space:toggle checkbox  esc:cancel",
            RuleDialogMode::RemovePattern => "  << Remove Pattern >>  enter:confirm  esc:cancel",
            RuleDialogMode::Numbering => match app.rule_input_step {
                RuleInputStep::InputNumber => "  Start number: enter:next  esc:cancel",
                RuleInputStep::InputWidth => "  Width (digits): enter:next  esc:cancel",
                RuleInputStep::InputPlaceholder => {
                    "  Placeholder (e.g. ##): enter:confirm  esc:cancel"
                }
                _ => "  << Numbering >>  enter:confirm  esc:cancel",
            },
            RuleDialogMode::Case => {
                "  << Change Case >>  1:UPPER  2:lower  3:Title  4:tOGGLE  esc:cancel"
            }
            RuleDialogMode::None => unreachable!(),
        }
    } else {
        &" arrows:navigate  tab:panel  enter:Add Rule  d:del  ctrl+w:rename  q:quit"
    };

    let msg = if let Some(ref err) = app.error_msg {
        format!("{} | ERR: {}", app.status_msg, err)
    } else {
        app.status_msg.clone()
    };

    let status = Paragraph::new(Line::from(Span::styled(
        format!(
            " {}{} ",
            msg,
            help_text
        ),
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )));

    frame.render_widget(status, area);
}

fn handle_key(app: &mut App, key: KeyEvent, visible_height: u16) {
    if app.dialog_mode != RuleDialogMode::None {
        handle_dialog_key(app, key);
        return;
    }

    match key.code {
        event::KeyCode::Char('q') => {
            if app.rules.is_empty() {
                app.running = false;
            } else {
                app.dialog_mode = RuleDialogMode::ConfirmQuit;
                app.dialog_cursor = 0;
                app.rule_input_buffer.clear();
                app.rule_input_step = RuleInputStep::Waiting;
            }
        }
        event::KeyCode::Tab => {
            app.active_panel = if app.active_panel == Panel::Rules {
                Panel::Files
            } else {
                Panel::Rules
            };
            app.clear_input();
        }
        _ => {
            if app.active_panel == Panel::Rules {
                handle_rules_key(app, key);
            } else {
                handle_files_key(app, key, visible_height);
            }
        }
    }
}

fn handle_rules_key(app: &mut App, key: KeyEvent) {
    let total_items = app.rules.len() + 1;

    match key.code {
        event::KeyCode::Down => {
            if app.rule_cursor < total_items.saturating_sub(1) {
                app.rule_cursor += 1;
            }
        }
        event::KeyCode::Up => {
            if app.rule_cursor > 0 {
                app.rule_cursor -= 1;
            }
        }
        event::KeyCode::Char('d') => {
            if app.rule_cursor < app.rules.len() {
                app.remove_rule();
            }
        }
        event::KeyCode::Enter => {
            if app.rule_cursor == app.rules.len() {
                app.dialog_mode = RuleDialogMode::SelectRule;
                app.dialog_cursor = 0;
                app.rule_input_buffer.clear();
                app.rule_input_step = RuleInputStep::Waiting;
            }
        }
        _ => {}
    }
}

fn handle_dialog_key(app: &mut App, key: KeyEvent) {
    match app.dialog_mode {
        RuleDialogMode::ConfirmQuit => match key.code {
            event::KeyCode::Down | event::KeyCode::Char('n') => {
                if app.dialog_cursor < 1 {
                    app.dialog_cursor = 1;
                }
            }
            event::KeyCode::Up | event::KeyCode::Char('y') => {
                if app.dialog_cursor > 0 {
                    app.dialog_cursor = 0;
                }
            }
            event::KeyCode::Enter => {
                if app.dialog_cursor == 0 {
                    app.running = false;
                    app.dialog_mode = RuleDialogMode::None;
                } else {
                    app.clear_input();
                }
            }
            event::KeyCode::Esc => {
                app.clear_input();
            }
            _ => {}
        },
        RuleDialogMode::SelectRule => match key.code {
            event::KeyCode::Down => {
                if app.dialog_cursor < 6 {
                    app.dialog_cursor += 1;
                }
            }
            event::KeyCode::Up => {
                if app.dialog_cursor > 0 {
                    app.dialog_cursor -= 1;
                }
            }
            event::KeyCode::Enter => {
                let rule = match app.dialog_cursor {
                    0 => {
                        app.dialog_mode = RuleDialogMode::FindReplace;
                        app.rule_input_step = RuleInputStep::InputText;
                        app.rule_input_buffer.clear();
                        None
                    }
                    1 => {
                        app.dialog_mode = RuleDialogMode::FindReplaceRegex;
                        app.rule_input_step = RuleInputStep::InputText;
                        app.rule_input_buffer.clear();
                        None
                    }
                    2 => {
                        app.dialog_mode = RuleDialogMode::Prefix;
                        app.rule_input_step = RuleInputStep::InputText;
                        app.rule_input_buffer.clear();
                        None
                    }
                    3 => {
                        app.dialog_mode = RuleDialogMode::Case;
                        app.rule_input_step = RuleInputStep::SelectCase;
                        app.rule_input_buffer.clear();
                        None
                    }
                    4 => {
                        app.dialog_mode = RuleDialogMode::RemovePattern;
                        app.rule_input_step = RuleInputStep::InputText;
                        app.rule_input_buffer.clear();
                        None
                    }
                    5 => {
                        app.dialog_mode = RuleDialogMode::Numbering;
                        app.rule_input_step = RuleInputStep::InputNumber;
                        app.rule_input_buffer.clear();
                        None
                    }
                    _ => None,
                };
                if let Some(r) = rule {
                    app.add_rule(r);
                    app.clear_input();
                }
            }
            event::KeyCode::Char('1') => {
                if app.dialog_cursor != 0 {
                    app.dialog_cursor = 0;
                } else {
                    app.dialog_mode = RuleDialogMode::FindReplaceRegex;
                    app.rule_input_step = RuleInputStep::InputText;
                    app.rule_input_buffer.clear();
                }
            }
            event::KeyCode::Char('2') => {
                if app.dialog_cursor != 1 {
                    app.dialog_cursor = 1;
                } else {
                    app.dialog_mode = RuleDialogMode::FindReplace;
                    app.rule_input_step = RuleInputStep::InputText;
                    app.rule_input_buffer.clear();
                }
            }
            event::KeyCode::Char('3') => {
                if app.dialog_cursor != 2 {
                    app.dialog_cursor = 2;
                } else {
                    app.dialog_mode = RuleDialogMode::Prefix;
                    app.rule_input_step = RuleInputStep::InputText;
                    app.rule_input_buffer.clear();
                }
            }
            event::KeyCode::Char('4') => {
                if app.dialog_cursor != 3 {
                    app.dialog_cursor = 3;
                } else {
                    app.dialog_mode = RuleDialogMode::Suffix;
                    app.rule_input_step = RuleInputStep::InputText;
                    app.rule_input_buffer.clear();
                }
            }
            event::KeyCode::Char('5') => {
                if app.dialog_cursor != 4 {
                    app.dialog_cursor = 4;
                } else {
                    app.dialog_mode = RuleDialogMode::Case;
                    app.rule_input_step = RuleInputStep::SelectCase;
                    app.rule_input_buffer.clear();
                }
            }
            event::KeyCode::Char('6') => {
                if app.dialog_cursor != 5 {
                    app.dialog_cursor = 5;
                } else {
                    app.dialog_mode = RuleDialogMode::RemovePattern;
                    app.rule_input_step = RuleInputStep::InputText;
                    app.rule_input_buffer.clear();
                }
            }
            event::KeyCode::Char('7') => {
                if app.dialog_cursor != 6 {
                    app.dialog_cursor = 6;
                } else {
                    app.dialog_mode = RuleDialogMode::Numbering;
                    app.rule_input_step = RuleInputStep::InputNumber;
                    app.rule_input_buffer.clear();
                }
            }
            event::KeyCode::Esc => {
                app.clear_input();
            }
            event::KeyCode::Char('q') => {
                app.clear_input();
            }
            _ => {}
        },
        _ => {
            if key.code == event::KeyCode::Char('q') {
                app.clear_input();
            } else {
                handle_rule_input_key(app, key);
            }
        }
    }
}

fn handle_files_key(app: &mut App, key: KeyEvent, visible_height: u16) {
    match key.code {
        event::KeyCode::Down => {
            if !app.files.is_empty() && app.file_cursor < app.files.len().saturating_sub(1) {
                app.file_cursor += 1;
            }
            let vf = visible_height as usize / 2;
            if app.file_cursor >= app.file_scroll + vf {
                app.file_scroll = app.file_cursor - vf + 1;
            }
        }
        event::KeyCode::Up => {
            if app.file_cursor > 0 {
                app.file_cursor -= 1;
            }
            if app.file_cursor < app.file_scroll {
                app.file_scroll = app.file_cursor;
            }
        }
        event::KeyCode::Char('g') => {
            app.file_cursor = 0;
            app.file_scroll = 0;
        }
        event::KeyCode::Char('G') => {
            if !app.files.is_empty() {
                app.file_cursor = app.files.len() - 1;
                app.file_scroll = app.file_cursor;
            }
        }
        event::KeyCode::Char('d') => {
            let vf = visible_height as usize / 2;
            if app.file_cursor + vf < app.files.len() {
                app.file_scroll = (app.file_scroll + vf).min(app.files.len().saturating_sub(1));
                app.file_cursor = app.file_scroll;
            }
        }
        event::KeyCode::Char('u') => {
            let vf = visible_height as usize / 2;
            if app.file_scroll > 0 {
                app.file_scroll = app.file_scroll.saturating_sub(vf);
                app.file_cursor = app.file_scroll;
            }
        }

        event::KeyCode::Char(c)
            if key.modifiers.contains(event::KeyModifiers::CONTROL) && c == 'w' =>
        {
            let results = app.rename_files();
            app.status_msg = format!("Renamed {} files.", results.len());
            app.running = false;
        }
        event::KeyCode::Char('/') => {
            app.refresh_files();
            app.status_msg = String::from("File list refreshed.");
        }
        _ => {}
    }
}

fn handle_rule_input_key(app: &mut App, key: KeyEvent) {
    match key.code {
        event::KeyCode::Esc => {
            app.clear_input();
        }
        event::KeyCode::Enter => {
            submit_rule_input(app);
        }
        event::KeyCode::Backspace => {
            let _ = app.rule_input_buffer.pop();
        }
        event::KeyCode::Char(c) => {
            if app.rule_input_step == RuleInputStep::SelectCase {
                match c {
                    '1' => {
                        app.add_rule(RenameRule::ChangeCase(CaseTransform::Upper));
                        app.clear_input();
                    }
                    '2' => {
                        app.add_rule(RenameRule::ChangeCase(CaseTransform::Lower));
                        app.clear_input();
                    }
                    '3' => {
                        app.add_rule(RenameRule::ChangeCase(CaseTransform::Title));
                        app.clear_input();
                    }
                    '4' => {
                        app.add_rule(RenameRule::ChangeCase(CaseTransform::Toggle));
                        app.clear_input();
                    }
                    _ => {}
                }
            } else if app.dialog_mode == RuleDialogMode::Suffix && c == ' ' {
                app.suffix_after_extension = !app.suffix_after_extension;
            } else {
                app.rule_input_buffer.push(c);
            }
        }
        _ => {}
    }
}

fn submit_rule_input(app: &mut App) {
    let buf = app.rule_input_buffer.clone();

    match (app.dialog_mode, app.rule_input_step) {
        (RuleDialogMode::FindReplace, RuleInputStep::InputText)
        | (RuleDialogMode::FindReplaceRegex, RuleInputStep::InputText) => {
            app.find_pattern = Some(buf);
            app.rule_input_step = RuleInputStep::InputReplace;
            app.rule_input_buffer.clear();
        }
        (RuleDialogMode::FindReplace, RuleInputStep::InputReplace)
        | (RuleDialogMode::FindReplaceRegex, RuleInputStep::InputReplace) => {
            let is_regex = app.dialog_mode == RuleDialogMode::FindReplaceRegex;
            if let Some(find) = app.find_pattern.take() {
                if is_regex {
                    app.add_rule(RenameRule::FindReplaceRegex {
                        pattern: find,
                        replacement: buf,
                    });
                } else {
                    app.add_rule(RenameRule::FindReplace { find, replace: buf });
                }
                app.clear_input();
            }
        }
        (RuleDialogMode::Prefix, RuleInputStep::InputText) => {
            app.add_rule(RenameRule::AddPrefix(buf));
            app.clear_input();
        }
        (RuleDialogMode::Suffix, RuleInputStep::InputText) => {
            let after_ext = app.suffix_after_extension;
            app.add_rule(RenameRule::AddSuffix {
                text: buf,
                after_extension: after_ext,
            });
            app.clear_input();
        }
        (RuleDialogMode::RemovePattern, RuleInputStep::InputText) => {
            app.add_rule(RenameRule::RemovePattern(buf));
            app.clear_input();
        }
        (RuleDialogMode::Numbering, RuleInputStep::InputNumber) => {
            app.numbering_start = buf.parse().ok();
            app.rule_input_step = RuleInputStep::InputWidth;
            app.rule_input_buffer.clear();
        }
        (RuleDialogMode::Numbering, RuleInputStep::InputWidth) => {
            app.numbering_width = buf.parse().ok();
            app.rule_input_step = RuleInputStep::InputPlaceholder;
            app.rule_input_buffer.clear();
        }
        (RuleDialogMode::Numbering, RuleInputStep::InputPlaceholder) => {
            if let (Some(start), Some(width)) = (app.numbering_start, app.numbering_width) {
                app.add_rule(RenameRule::Numbering {
                    start,
                    width,
                    placeholder: buf,
                });
                app.clear_input();
            }
        }
        _ => {
            app.clear_input();
        }
    }
}
