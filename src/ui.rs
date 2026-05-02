use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Terminal;

use crate::app::{App, Panel, RuleInputMode, RuleInputStep};
use crate::rules::{apply_rules, CaseTransform, RenameRule};

pub fn run(mut app: App) -> color_eyre::Result<()> {
    let mut terminal = init_terminal()?;
    let result = main_loop(&mut app, &mut terminal);
    restore_terminal();
    result
}

fn init_terminal() -> color_eyre::Result<Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>> {
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
        terminal.draw(|frame| render(frame, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(app, key);
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
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)].as_ref())
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
        Style::default().fg(Color::Cyan).add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(" Rename Rules ", title_style)))
        .border_style(border_style);

    let rows: Vec<Row> = app
        .rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let prefix = if i == app.rule_cursor { ">> " } else { "   " };
            Row::new(vec![format!("{}[{}] {}", prefix, i, rule)])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Min(0)]).block(block);
    frame.render_widget(table, area);

    if app.rules.is_empty() && app.rule_input_mode == RuleInputMode::None {
        let hint = Paragraph::new(vec![
            Line::from(Span::styled("No rules yet.", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled("Add rule (while in rules panel):", Style::default().fg(Color::Gray))),
            Line::from(Span::styled("  f  find+replace", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("  p  add prefix", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("  s  add suffix", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("  c  change case", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("  r  remove pattern", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("  n  numbering", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled("  d  delete rule  J/K  move rule", Style::default().fg(Color::DarkGray))),
        ]);
        frame.render_widget(hint, area);
    }

    if app.rule_input_mode != RuleInputMode::None {
        render_rule_input(frame, area, app);
    }
}

fn render_rule_input(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let prompt = match (app.rule_input_mode, app.rule_input_step) {
        (RuleInputMode::FindReplace, RuleInputStep::InputText) => "Find pattern:",
        (RuleInputMode::FindReplace, RuleInputStep::InputReplace) => "Replace with:",
        (RuleInputMode::FindReplace, RuleInputStep::ConfirmRegex) => "Use regex? (y/n):",
        (RuleInputMode::Prefix, RuleInputStep::InputText) => "Prefix string:",
        (RuleInputMode::Suffix, RuleInputStep::InputText) => "Suffix string:",
        (RuleInputMode::RemovePattern, RuleInputStep::InputText) => "Pattern to remove:",
        (RuleInputMode::Numbering, RuleInputStep::InputNumber) => "Start number:",
        (RuleInputMode::Numbering, RuleInputStep::InputWidth) => "Width (digits):",
        (RuleInputMode::Numbering, RuleInputStep::InputPlaceholder) => "Placeholder (e.g. ##):",
        (RuleInputMode::Case, RuleInputStep::SelectCase) => "Case: 1=UPPER 2=lower 3=Title 4=tOGGLE",
        _ => "Input:",
    };

    let input_text = format!("> {} {}|", prompt, app.rule_input_buffer);
    let input_para = Paragraph::new(input_text).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let input_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };

    frame.render_widget(input_para, input_area);
}

fn render_files_panel(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Files;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_style = if is_active {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::REVERSED)
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

    let mut rows = Vec::new();

    for (i, file) in app.files.iter().enumerate() {
        let new_name = apply_rules(file, &app.rules, i as u32);
        let preview = if new_name != *file {
            format!("{} -> {}", file, new_name)
        } else {
            format!("{}", file)
        };

        let style = if i == app.file_cursor {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        rows.push(Row::new(vec![preview]).style(style));
    }

    let table = Table::new(rows, [Constraint::Min(0)]).block(block);
    frame.render_widget(table, area);
}



fn render_statusbar(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let msg = if let Some(ref err) = app.error_msg {
        format!("{} | ERR: {}", app.status_msg, err)
    } else {
        app.status_msg.clone()
    };

    let status = Paragraph::new(Line::from(Span::styled(
        format!(" {} | Files: {} | Rules: {} ", msg, app.files.len(), app.rules.len()),
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )));

    frame.render_widget(status, area);
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if app.rule_input_mode != RuleInputMode::None {
        handle_rule_input_key(app, key);
        return;
    }

    match key.code {
        event::KeyCode::Char('q') => {
            app.running = false;
        }
        event::KeyCode::Char('h') | event::KeyCode::Char('l') | event::KeyCode::Tab => {
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
                handle_files_key(app, key);
            }
        }
    }
}

fn handle_rules_key(app: &mut App, key: KeyEvent) {
    if app.rule_input_mode != RuleInputMode::None {
        handle_rule_input_key(app, key);
        return;
    }

    match key.code {
        event::KeyCode::Char('j') | event::KeyCode::Down => {
            if app.rule_cursor < app.rules.len().saturating_sub(1) {
                app.rule_cursor += 1;
            }
        }
        event::KeyCode::Char('k') | event::KeyCode::Up => {
            if app.rule_cursor > 0 {
                app.rule_cursor -= 1;
            }
        }
        event::KeyCode::Char('d') => {
            app.remove_rule();
        }
        event::KeyCode::Char('J') => {
            app.move_rule_down();
        }
        event::KeyCode::Char('K') => {
            app.move_rule_up();
        }
        event::KeyCode::Char('f') => {
            app.rule_input_mode = RuleInputMode::FindReplace;
            app.rule_input_step = RuleInputStep::InputText;
        }
        event::KeyCode::Char('p') => {
            app.rule_input_mode = RuleInputMode::Prefix;
            app.rule_input_step = RuleInputStep::InputText;
        }
        event::KeyCode::Char('s') => {
            app.rule_input_mode = RuleInputMode::Suffix;
            app.rule_input_step = RuleInputStep::InputText;
        }
        event::KeyCode::Char('c') => {
            app.rule_input_mode = RuleInputMode::Case;
            app.rule_input_step = RuleInputStep::SelectCase;
        }
        event::KeyCode::Char('r') => {
            app.rule_input_mode = RuleInputMode::RemovePattern;
            app.rule_input_step = RuleInputStep::InputText;
        }
        event::KeyCode::Char('n') => {
            app.rule_input_mode = RuleInputMode::Numbering;
            app.rule_input_step = RuleInputStep::InputNumber;
        }
        _ => {}
    }
}

fn handle_files_key(app: &mut App, key: KeyEvent) {
    match key.code {
        event::KeyCode::Char('j') | event::KeyCode::Down => {
            if !app.files.is_empty() && app.file_cursor < app.files.len().saturating_sub(1) {
                app.file_cursor += 1;
            }
        }
        event::KeyCode::Char('k') | event::KeyCode::Up => {
            if app.file_cursor > 0 {
                app.file_cursor -= 1;
            }
        }
        event::KeyCode::Char('g') => {

        }
        event::KeyCode::Char('G') => {
            if !app.files.is_empty() {
                app.file_cursor = app.files.len() - 1;
            }
        }

        event::KeyCode::Char('r') => {
            let results = app.rename_files(true);
            app.status_msg = format!("Dry-run: {} files would be renamed.", results.len());
        }
        event::KeyCode::Char('R') => {
            let results = app.rename_files(false);
            app.refresh_files();
            app.status_msg = format!("Renamed {} files.", results.len());
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
            } else {
                app.rule_input_buffer.push(c);
            }
        }
        _ => {}
    }
}

fn submit_rule_input(app: &mut App) {
    let buf = app.rule_input_buffer.clone();

    match (app.rule_input_mode, app.rule_input_step) {
        (RuleInputMode::FindReplace, RuleInputStep::InputText) => {
            app.find_replace_find = Some(buf);
            app.rule_input_step = RuleInputStep::InputReplace;
            app.rule_input_buffer.clear();
        }
        (RuleInputMode::FindReplace, RuleInputStep::InputReplace) => {
            app.rule_input_step = RuleInputStep::ConfirmRegex;
            app.rule_input_buffer.clear();
        }
        (RuleInputMode::FindReplace, RuleInputStep::ConfirmRegex) => {
            if let Some(find) = app.find_replace_find.take() {
                let is_regex = buf.chars().next().map(|c| c == 'y' || c == 'Y').unwrap_or(false);
                app.add_rule(RenameRule::FindReplace {
                    find,
                    replace: buf,
                    regex: is_regex,
                });
                app.clear_input();
            }
        }
        (RuleInputMode::Prefix, RuleInputStep::InputText) => {
            app.add_rule(RenameRule::AddPrefix(buf));
            app.clear_input();
        }
        (RuleInputMode::Suffix, RuleInputStep::InputText) => {
            app.add_rule(RenameRule::AddSuffix(buf));
            app.clear_input();
        }
        (RuleInputMode::RemovePattern, RuleInputStep::InputText) => {
            app.add_rule(RenameRule::RemovePattern(buf));
            app.clear_input();
        }
        (RuleInputMode::Numbering, RuleInputStep::InputNumber) => {
            app.numbering_start = buf.parse().ok();
            app.rule_input_step = RuleInputStep::InputWidth;
            app.rule_input_buffer.clear();
        }
        (RuleInputMode::Numbering, RuleInputStep::InputWidth) => {
            app.numbering_width = buf.parse().ok();
            app.rule_input_step = RuleInputStep::InputPlaceholder;
            app.rule_input_buffer.clear();
        }
        (RuleInputMode::Numbering, RuleInputStep::InputPlaceholder) => {
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
