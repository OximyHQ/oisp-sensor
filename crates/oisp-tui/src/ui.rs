//! TUI rendering

use crate::app::{App, View};
use oisp_core::events::OispEvent;
use ratatui::{prelude::*, widgets::*};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);

    match app.view {
        View::Timeline => draw_timeline(frame, chunks[1], app),
        View::Inventory => draw_inventory(frame, chunks[1], app),
        View::ProcessTree => draw_process_tree(frame, chunks[1], app),
        View::Traces => draw_traces(frame, chunks[1], app),
    }

    draw_footer(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " OISP Sensor | Events: {} | AI: {} ",
        app.total_events, app.ai_events
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(block, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let tabs = [
        ("t", "Timeline", app.view == View::Timeline),
        ("i", "Inventory", app.view == View::Inventory),
        ("p", "Process Tree", app.view == View::ProcessTree),
        ("r", "Traces", app.view == View::Traces),
    ];

    let spans: Vec<Span> = tabs
        .iter()
        .flat_map(|(key, name, active)| {
            let style = if *active {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            };
            vec![
                Span::styled(format!("[{}]", key), Style::default().fg(Color::Yellow)),
                Span::styled(format!(" {} ", name), style),
                Span::raw("  "),
            ]
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn draw_timeline(frame: &mut Frame, area: Rect, app: &App) {
    let events: Vec<ListItem> = app
        .timeline
        .iter()
        .skip(app.scroll)
        .take(area.height as usize - 2)
        .map(|event| {
            let (symbol, color) = match event.as_ref() {
                OispEvent::AiRequest(_) => (">>", Color::Green),
                OispEvent::AiResponse(_) => ("<<", Color::Blue),
                OispEvent::AgentToolCall(_) => ("->", Color::Yellow),
                OispEvent::AgentToolResult(_) => ("<-", Color::Yellow),
                OispEvent::ProcessExec(_) => ("*", Color::Magenta),
                OispEvent::FileWrite(_) => ("W", Color::Red),
                OispEvent::NetworkConnect(_) => ("@", Color::Cyan),
                _ => (".", Color::Gray),
            };

            let envelope = event.envelope();
            let ts = envelope.ts.format("%H:%M:%S%.3f");
            let event_type = event.event_type();
            let proc = envelope
                .process
                .as_ref()
                .and_then(|p| p.name.clone())
                .unwrap_or_else(|| "?".to_string());

            let detail = match event.as_ref() {
                OispEvent::AiRequest(e) => {
                    let provider = e
                        .data
                        .provider
                        .as_ref()
                        .map(|p| p.name.as_str())
                        .unwrap_or("?");
                    let model = e.data.model.as_ref().map(|m| m.id.as_str()).unwrap_or("?");
                    format!("{} {}", provider, model)
                }
                OispEvent::AiResponse(e) => {
                    let tokens = e
                        .data
                        .usage
                        .as_ref()
                        .and_then(|u| u.total_tokens)
                        .map(|t| format!("{}tok", t))
                        .unwrap_or_default();
                    let latency = e
                        .data
                        .latency_ms
                        .map(|l| format!("{}ms", l))
                        .unwrap_or_default();
                    format!("{} {}", tokens, latency)
                }
                OispEvent::AgentToolCall(e) => e.data.tool_name.clone(),
                OispEvent::ProcessExec(e) => e.data.exe.clone(),
                OispEvent::FileWrite(e) => e.data.path.clone(),
                OispEvent::NetworkConnect(e) => e
                    .data
                    .dest
                    .domain
                    .clone()
                    .or_else(|| e.data.dest.ip.clone())
                    .unwrap_or_else(|| "?".to_string()),
                _ => String::new(),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", symbol), Style::default().fg(color)),
                Span::styled(format!("{} ", ts), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:<18} ", event_type), Style::default().fg(color)),
                Span::styled(format!("{:<12} ", proc), Style::default().fg(Color::White)),
                Span::raw(detail),
            ]))
        })
        .collect();

    let list = List::new(events).block(Block::default().title(" Timeline ").borders(Borders::ALL));

    frame.render_widget(list, area);
}

fn draw_inventory(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Providers
    let provider_items: Vec<ListItem> = app
        .providers
        .values()
        .map(|p| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<15}", p.name), Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{:>6} req  ", p.request_count),
                    Style::default().fg(Color::White),
                ),
                Span::raw(p.models.join(", ")),
            ]))
        })
        .collect();

    let providers_list = List::new(provider_items).block(
        Block::default()
            .title(" AI Providers ")
            .borders(Borders::ALL),
    );

    frame.render_widget(providers_list, chunks[0]);

    // Apps
    let app_items: Vec<ListItem> = app
        .apps
        .values()
        .map(|a| {
            let account_style = match a.account_type.as_str() {
                "corporate" => Style::default().fg(Color::Green),
                "personal" => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::Gray),
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<15}", a.name),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("{:>6} req  ", a.request_count),
                    Style::default().fg(Color::White),
                ),
                Span::styled(format!("{:<10}", a.account_type), account_style),
            ]))
        })
        .collect();

    let apps_list = List::new(app_items).block(
        Block::default()
            .title(" Apps Using AI ")
            .borders(Borders::ALL),
    );

    frame.render_widget(apps_list, chunks[1]);
}

fn draw_process_tree(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .title(" Process Tree ")
        .borders(Borders::ALL);

    let text = "Process tree view - coming soon";
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}

fn draw_traces(frame: &mut Frame, area: Rect, app: &App) {
    let traces = app.traces();

    let items: Vec<ListItem> = traces
        .iter()
        .take(area.height as usize - 2)
        .map(|trace| {
            let duration = trace.duration();
            let status = if trace.is_complete { "done" } else { "..." };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", status),
                    Style::default().fg(if trace.is_complete {
                        Color::Green
                    } else {
                        Color::Yellow
                    }),
                ),
                Span::styled(
                    format!("{:<12} ", trace.process_name.as_deref().unwrap_or("?")),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:>5}ms ", duration.num_milliseconds()),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>6}tok ", trace.total_tokens),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{} LLM calls ", trace.llm_call_count),
                    Style::default().fg(Color::Blue),
                ),
                Span::styled(
                    format!("{} tools", trace.tool_call_count),
                    Style::default().fg(Color::Magenta),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Agent Traces ")
            .borders(Borders::ALL),
    );

    frame.render_widget(list, area);
}
