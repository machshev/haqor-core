use std::io;

use anyhow::Result;
use chrono::{Local, TimeZone};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use haqor_core::tutor::IssueReport;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub(super) enum Action {
    Pull,
    Sync(Vec<String>),
}

pub(super) struct ActionResult {
    pub reports: Vec<IssueReport>,
    pub resolved: usize,
    pub message: String,
}

/// Show reports and run pull/sync actions without leaving the TUI.
pub(super) fn review(
    mut reports: Vec<IssueReport>,
    mut action: impl FnMut(Action, &[IssueReport]) -> Result<ActionResult>,
) -> Result<usize> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut selected = 0_usize;
    let mut resolving = vec![false; reports.len()];
    let mut resolved_count = 0;
    let mut status =
        "↑/↓ or j/k move · d/space mark resolved · p pull · s sync · q quit".to_string();

    loop {
        terminal.draw(|frame| draw(frame, &reports, selected, &resolving, &status))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(resolved_count),
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') if !reports.is_empty() => {
                selected = (selected + 1).min(reports.len() - 1)
            }
            KeyCode::Char('d') | KeyCode::Char(' ') if !reports.is_empty() => {
                resolving[selected] = !resolving[selected]
            }
            KeyCode::Char('p') => match action(Action::Pull, &reports) {
                Ok(result) => {
                    reports = result.reports;
                    selected = 0;
                    resolving = vec![false; reports.len()];
                    status = result.message;
                }
                Err(error) => status = format!("Pull failed: {error:#}"),
            },
            KeyCode::Char('s') => {
                let ids = reports
                    .iter()
                    .zip(resolving.iter())
                    .filter(|(_, resolved)| **resolved)
                    .map(|(report, _)| report.id.clone())
                    .collect::<Vec<_>>();
                if ids.is_empty() {
                    status = "Nothing marked resolved".to_string();
                    continue;
                }
                match action(Action::Sync(ids), &reports) {
                    Ok(result) => {
                        resolved_count += result.resolved;
                        reports = result.reports;
                        selected = selected.min(reports.len().saturating_sub(1));
                        resolving = vec![false; reports.len()];
                        status = result.message;
                    }
                    Err(error) => status = format!("Sync failed: {error:#}"),
                }
            }
            _ => {}
        }
    }
}

fn draw(
    frame: &mut ratatui::Frame,
    reports: &[IssueReport],
    selected: usize,
    resolving: &[bool],
    status: &str,
) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(60),
            Constraint::Length(2),
        ])
        .split(frame.area());
    let items = reports
        .iter()
        .enumerate()
        .map(|(index, report)| {
            let marker = if resolving[index] {
                "[resolve]"
            } else {
                "[       ]"
            };
            let note = report.note.lines().next().unwrap_or_default();
            let type_color = if report.report_type == "bug" {
                Color::Red
            } else {
                Color::Cyan
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    marker,
                    Style::default().fg(if resolving[index] {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::raw(" "),
                Span::styled(
                    format_timestamp(report.created_epoch),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:<4}", report.report_type),
                    Style::default().fg(type_color),
                ),
                Span::raw(" "),
                Span::raw(note.to_string()),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" App issue reports "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, areas[0], &mut state);

    if reports.is_empty() {
        frame.render_widget(
            Paragraph::new("No active app issue reports.").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Selected report "),
            ),
            areas[1],
        );
        frame.render_widget(Paragraph::new(status), areas[2]);
        return;
    }
    let report = &reports[selected];
    let type_style = match report.report_type.as_str() {
        "bug" => Style::default()
            .fg(Color::Red)
            .add_modifier(ratatui::style::Modifier::BOLD),
        "idea" => Style::default()
            .fg(Color::Cyan)
            .add_modifier(ratatui::style::Modifier::BOLD),
        _ => Style::default().fg(Color::Yellow),
    };
    let mut detail_lines = vec![Line::from(vec![
        Span::styled(report.report_type.to_uppercase(), type_style),
        Span::raw("  "),
        Span::styled(
            format_timestamp(report.created_epoch),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  "),
        Span::styled(
            format!("id: {}", report.id),
            Style::default().fg(Color::DarkGray),
        ),
    ])];
    detail_lines.push(Line::from(vec![
        Span::styled("Note: ", Style::default().fg(Color::Yellow)),
        Span::raw(report.note.clone()),
    ]));
    detail_lines.push(Line::from(""));
    match serde_json::from_str::<serde_json::Value>(&report.context_json) {
        Ok(value) => append_context_lines(&mut detail_lines, &value, ""),
        Err(_) => detail_lines.push(Line::from(vec![
            Span::styled("Context: ", Style::default().fg(Color::Red)),
            Span::raw(report.context_json.clone()),
        ])),
    }
    let detail = Text::from(detail_lines);
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Selected report "),
            )
            .wrap(Wrap { trim: false }),
        areas[1],
    );
    frame.render_widget(Paragraph::new(status), areas[2]);
}

fn append_context_lines(lines: &mut Vec<Line<'static>>, value: &serde_json::Value, prefix: &str) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                // `details` is the wrapper used by the app around the useful
                // screen-specific context; don't make it part of every label.
                let label = if prefix.is_empty() || prefix == "details" {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                match value {
                    serde_json::Value::Object(_) => append_context_lines(lines, value, &label),
                    serde_json::Value::Array(_) => {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{label}: "), Style::default().fg(Color::Blue)),
                            Span::raw(compact_json(value)),
                        ]));
                    }
                    _ => lines.push(Line::from(vec![
                        Span::styled(format!("{label}: "), Style::default().fg(Color::Blue)),
                        Span::raw(compact_json(value)),
                    ])),
                }
            }
        }
        _ => lines.push(Line::from(compact_json(value))),
    }
}

fn compact_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "?".to_string()),
    }
}

fn format_timestamp(epoch: i64) -> String {
    Local
        .timestamp_opt(epoch, 0)
        .single()
        .map(|timestamp| timestamp.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| format!("epoch {epoch}"))
}

#[cfg(test)]
mod tests {
    use super::{append_context_lines, format_timestamp};
    use ratatui::text::Line;

    #[test]
    fn formats_created_epoch_for_the_review_table() {
        assert_ne!(format_timestamp(0), "epoch 0");
    }

    #[test]
    fn extracts_nested_context_fields_into_labeled_lines() {
        let value = serde_json::json!({
            "source": "word_info",
            "details": {"word": "אָב", "result": {"gloss": "father"}}
        });
        let mut lines: Vec<Line<'static>> = Vec::new();
        append_context_lines(&mut lines, &value, "");
        let rendered = lines
            .iter()
            .map(Line::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("source: word_info"));
        assert!(rendered.contains("result.gloss: father"));
        assert!(!rendered.contains("details:"));
        assert!(!rendered.contains("details.lookup:"));
    }
}
