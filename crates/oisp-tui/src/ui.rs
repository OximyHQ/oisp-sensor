//! TUI rendering

use crate::app::{App, ProcessNode, View};
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

    let mut spans: Vec<Span> = tabs
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

    // Add quit hint
    spans.push(Span::raw("  │  "));
    spans.push(Span::styled("[q]", Style::default().fg(Color::Red)));
    spans.push(Span::styled(" Quit ", Style::default().fg(Color::Gray)));
    spans.push(Span::raw("  "));
    spans.push(Span::styled("[j/k]", Style::default().fg(Color::Yellow)));
    spans.push(Span::styled(" Scroll", Style::default().fg(Color::Gray)));

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

            // Prefer app name from AppInfo, fall back to process name
            let app_display = envelope
                .app
                .as_ref()
                .and_then(|a| a.name.clone())
                .or_else(|| envelope.process.as_ref().and_then(|p| p.name.clone()))
                .unwrap_or_else(|| "?".to_string());
            // Truncate to 12 chars for display
            let app_display = if app_display.len() > 12 {
                format!("{}…", &app_display[..11])
            } else {
                app_display
            };

            let detail = match event.as_ref() {
                OispEvent::AiRequest(e) => {
                    let provider = e
                        .data
                        .provider
                        .as_ref()
                        .map(|p| p.name.as_str())
                        .unwrap_or("?");
                    let model = e.data.model.as_ref().map(|m| m.id.as_str()).unwrap_or("?");
                    // Show web app if present (browser-originated request)
                    let web_app = e
                        .envelope
                        .web_context
                        .as_ref()
                        .and_then(|ctx| ctx.web_app_name.as_deref())
                        .map(|name| format!(" (via {})", name))
                        .unwrap_or_default();
                    format!("{} {}{}", provider, model, web_app)
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
                OispEvent::AgentToolCall(e) => e.data.tool.name.clone().unwrap_or_default(),
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
                Span::styled(
                    format!("{:<12} ", app_display),
                    Style::default().fg(Color::White),
                ),
                Span::raw(detail),
            ]))
        })
        .collect();

    let list = List::new(events).block(Block::default().title(" Timeline ").borders(Borders::ALL));

    frame.render_widget(list, area);
}

fn draw_inventory(frame: &mut Frame, area: Rect, app: &App) {
    // Use vertical layout for providers on top, apps and web apps below
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
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

    frame.render_widget(providers_list, main_chunks[0]);

    // Bottom section: Apps and Web Apps side by side
    let bottom_chunks = if app.web_apps.is_empty() {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(main_chunks[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[1])
    };

    // Apps - sorted by request count descending
    let mut sorted_apps: Vec<_> = app.apps.values().collect();
    sorted_apps.sort_by(|a, b| b.request_count.cmp(&a.request_count));

    let app_items: Vec<ListItem> = sorted_apps
        .iter()
        .map(|a| {
            // Tier indicator: green for profiled, yellow for identified, red for unknown
            let (tier_symbol, tier_color) = match a.tier.as_str() {
                "profiled" => ("●", Color::Green), // Known app with full profile
                "identified" => ("○", Color::Yellow), // Matched but limited profile
                _ => ("?", Color::Red),            // Unknown - potentially suspicious
            };

            let account_style = match a.account_type.as_str() {
                "corporate" => Style::default().fg(Color::Green),
                "personal" => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::Gray),
            };

            // Truncate name to fit column
            let display_name = if a.name.len() > 16 {
                format!("{}…", &a.name[..15])
            } else {
                a.name.clone()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", tier_symbol), Style::default().fg(tier_color)),
                Span::styled(
                    format!("{:<16}", display_name),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("{:>5} req  ", a.request_count),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(
                        "{:<10}",
                        if a.account_type.is_empty() {
                            "-"
                        } else {
                            &a.account_type
                        }
                    ),
                    account_style,
                ),
            ]))
        })
        .collect();

    let apps_list = List::new(app_items).block(
        Block::default()
            .title(" Desktop Apps (● profiled ○ identified ? unknown) ")
            .borders(Borders::ALL),
    );

    frame.render_widget(apps_list, bottom_chunks[0]);

    // Web Apps - sorted by request count descending
    if !app.web_apps.is_empty() {
        let mut sorted_web_apps: Vec<_> = app.web_apps.values().collect();
        sorted_web_apps.sort_by(|a, b| b.request_count.cmp(&a.request_count));

        let web_app_items: Vec<ListItem> = sorted_web_apps
            .iter()
            .map(|wa| {
                // Type indicator: globe for direct, embed icon for embedded
                let (type_symbol, type_color) = match wa.web_app_type.as_str() {
                    "direct" => ("🌐", Color::Cyan),     // Direct API calls from web
                    "embedded" => ("⚡", Color::Yellow), // Embedded AI in web app
                    _ => ("○", Color::Gray),
                };

                // Truncate name to fit column
                let display_name = if wa.name.len() > 14 {
                    format!("{}…", &wa.name[..13])
                } else {
                    wa.name.clone()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", type_symbol), Style::default().fg(type_color)),
                    Span::styled(
                        format!("{:<14}", display_name),
                        Style::default().fg(Color::LightBlue),
                    ),
                    Span::styled(
                        format!("{:>5} req  ", wa.request_count),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw(wa.providers.join(", ")),
                ]))
            })
            .collect();

        let web_apps_list = List::new(web_app_items).block(
            Block::default()
                .title(" Web Apps (🌐 direct ⚡ embedded) ")
                .borders(Borders::ALL),
        );

        frame.render_widget(web_apps_list, bottom_chunks[1]);
    }
}

fn draw_process_tree(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(
            " Process Tree ({} processes) ",
            app.processes.len()
        ))
        .borders(Borders::ALL);

    if app.processes.is_empty() {
        let text = "No processes captured yet. Events will populate the tree.";
        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, area);
        return;
    }

    // Build flat list with indentation for tree display
    let mut lines: Vec<Line> = Vec::new();
    let root_processes = app.root_processes();

    // Sort roots by PID for consistent display
    let mut roots: Vec<&ProcessNode> = root_processes;
    roots.sort_by_key(|p| p.pid);

    for root in roots.iter().skip(app.scroll).take(area.height as usize - 2) {
        render_process_node(&mut lines, root, app, 0, area.height as usize - 2);
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Recursively render a process node and its children
fn render_process_node<'a>(
    lines: &mut Vec<Line<'a>>,
    node: &ProcessNode,
    app: &App,
    depth: usize,
    max_lines: usize,
) {
    if lines.len() >= max_lines {
        return;
    }

    // Build the tree prefix
    let prefix = if depth == 0 {
        String::new()
    } else {
        format!("{}├─ ", "│  ".repeat(depth - 1))
    };

    // Color based on AI activity
    let name_color = if node.ai_event_count > 0 {
        Color::Green
    } else {
        Color::White
    };

    let ai_indicator = if node.ai_event_count > 0 {
        format!(" [AI: {}]", node.ai_event_count)
    } else {
        String::new()
    };

    lines.push(Line::from(vec![
        Span::styled(prefix, Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", node.pid), Style::default().fg(Color::Cyan)),
        Span::raw(" "),
        Span::styled(node.name.clone(), Style::default().fg(name_color)),
        Span::styled(
            format!(" ({} events)", node.event_count),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(ai_indicator, Style::default().fg(Color::Green)),
    ]));

    // Render children
    let children = app.get_children(node.pid);
    let mut sorted_children: Vec<&ProcessNode> = children;
    sorted_children.sort_by_key(|p| p.pid);

    for child in sorted_children {
        render_process_node(lines, child, app, depth + 1, max_lines);
    }
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
