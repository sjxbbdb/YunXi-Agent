use crate::app::{YunxiTuiApp, YunxiTuiBanner};
use crate::bottom_pane::ApprovalRequestView;
use crate::render::render_tui_frame_with_styles;
use crate::styles::{TuiColorCapability, TuiStyleSet};
use crate::transcript_layout::build_wrapped_transcript;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use unicode_width::UnicodeWidthStr;
use yunxi_agent_core::{
    AgentEvent, AgentMessageSequence, AgentMessageStream, AgentMessageStreamPhase, CommandStatus,
};

struct IntegratedFixture {
    name: &'static str,
    width: u16,
    height: u16,
    capability: TuiColorCapability,
    capability_label: &'static str,
    private_marker: &'static str,
    app: YunxiTuiApp,
}

fn banner(provider_live: bool) -> YunxiTuiBanner {
    YunxiTuiBanner {
        cwd: "C:/Workspace/云汐/集成回归".to_string(),
        backend: "yunxi".to_string(),
        provider_live,
        provider_source: if provider_live {
            "fixture_live"
        } else {
            "offline_static"
        }
        .to_string(),
        model: "deepseek-chat".to_string(),
        provider: "deepseek".to_string(),
    }
}

fn historical_v210_app() -> YunxiTuiApp {
    let mut app = YunxiTuiApp::default();
    app.set_version_for_snapshot("v2.1.0");
    app
}

fn stream_event(content: &str, sequence: u64, phase: AgentMessageStreamPhase) -> AgentEvent {
    AgentEvent::Message {
        content: content.to_string(),
        stream: Some(AgentMessageStream {
            thread_id: "fixture-thread".to_string(),
            turn_id: "fixture-turn".to_string(),
            stream_id: "fixture-stream".to_string(),
            event_id: format!("fixture-event-{sequence}"),
            source_sequence: AgentMessageSequence::ProviderReliable(sequence),
            phase,
        }),
    }
}

fn normal_companion_fixture() -> IntegratedFixture {
    let mut app = historical_v210_app();
    app.set_banner(banner(false));
    app.push_user("请用三点总结今天的开发进度。".to_string());
    app.push_agent_event(&stream_event(
        "当然可以：\n\n1. 已固定终端模式选择。\n2. 已加固流式恢复。\n3. 正在执行集成发布回归。",
        1,
        AgentMessageStreamPhase::Final,
    ));
    app.push_agent_event(&AgentEvent::Reasoning {
        content: "PRIVATE_NORMAL_CONTEXT".to_string(),
    });
    IntegratedFixture {
        name: "normal-companion",
        width: 100,
        height: 30,
        capability: TuiColorCapability::Full,
        capability_label: "full",
        private_marker: "PRIVATE_NORMAL_CONTEXT",
        app,
    }
}

fn long_stream_fixture() -> IntegratedFixture {
    let mut app = historical_v210_app();
    app.set_banner(banner(true));
    app.push_user("生成包含 Markdown、中文与 Emoji 的长说明。".to_string());
    let content = (0..18)
        .map(|index| {
            format!(
                "## 阶段 {index:02}\n- 状态：稳定 ✅\n- 路径：`C:/Workspace/云汐/阶段-{index:02}`\n- 说明：流式 Markdown 保持单一消息单元，Emoji 👩‍💻 与组合字符 é 不破坏布局。"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    app.push_agent_event(&stream_event(&content, 1, AgentMessageStreamPhase::Delta));
    app.push_agent_event(&AgentEvent::Reasoning {
        content: "PRIVATE_LONG_STREAM_TRACE".to_string(),
    });
    IntegratedFixture {
        name: "long-stream-markdown-cjk",
        width: 58,
        height: 18,
        capability: TuiColorCapability::Full,
        capability_label: "full",
        private_marker: "PRIVATE_LONG_STREAM_TRACE",
        app,
    }
}

fn tool_approval_failure_fixture() -> IntegratedFixture {
    let mut app = historical_v210_app();
    app.set_banner(banner(true));
    app.push_user("检查生成目录。".to_string());
    app.push_agent_event(&AgentEvent::ToolCallStarted {
        id: Some("fixture-tool".to_string()),
        name: "shell".to_string(),
        arguments_json: Some("{\"path\":\"PRIVATE_TOOL_ARGUMENT\"}".to_string()),
    });
    app.push_agent_event(&AgentEvent::ToolCallCompleted {
        id: Some("fixture-tool".to_string()),
        name: "shell".to_string(),
        output: "PRIVATE_TOOL_FAILURE_OUTPUT".to_string(),
        status: CommandStatus::Failed,
    });
    app.start_approval(ApprovalRequestView {
        id: Some("fixture-approval".to_string()),
        tool_name: "shell".to_string(),
        cwd: "C:/Workspace/云汐".to_string(),
        command: Some("Remove-Item -Recurse C:/Workspace/云汐/generated".to_string()),
        reason: "删除生成目录前需要明确批准".to_string(),
        risk_label: Some("risk: destructive".to_string()),
    });
    IntegratedFixture {
        name: "tool-approval-failure",
        width: 80,
        height: 24,
        capability: TuiColorCapability::Full,
        capability_label: "full",
        private_marker: "PRIVATE_TOOL_FAILURE_OUTPUT",
        app,
    }
}

fn history_resize_fixture() -> IntegratedFixture {
    let mut app = historical_v210_app();
    app.set_banner(banner(false));
    for index in 0..36 {
        app.push_notice(
            "history",
            &format!("历史记录 {index:02}：窗口缩放后保持当前锚点。"),
        );
    }
    let narrow = build_wrapped_transcript(app.transcript().cells(), 56);
    app.scroll_up(10, &narrow, 10);
    let wide = build_wrapped_transcript(app.transcript().cells(), 96);
    app.reanchor_viewport(&wide, 18);
    app.push_notice("history", "缩放后到达的新输出");
    app.push_agent_event(&AgentEvent::Reasoning {
        content: "PRIVATE_RESIZE_ANCHOR".to_string(),
    });
    IntegratedFixture {
        name: "history-scroll-resize",
        width: 100,
        height: 30,
        capability: TuiColorCapability::Ansi16,
        capability_label: "ansi16",
        private_marker: "PRIVATE_RESIZE_ANCHOR",
        app,
    }
}

fn stream_fault_fixture() -> IntegratedFixture {
    let mut app = historical_v210_app();
    app.set_banner(banner(true));
    app.push_user("验证断流恢复。".to_string());
    app.push_agent_event(&stream_event(
        "已接收部分响应，等待后续数据……",
        1,
        AgentMessageStreamPhase::Delta,
    ));
    app.push_agent_event(&AgentEvent::ProviderError {
        provider: "deepseek".to_string(),
        status: Some(502),
        classification: "stream_disconnected".to_string(),
        message: "PRIVATE_PROVIDER_WIRE_BODY\nfixture connection reset".to_string(),
    });
    IntegratedFixture {
        name: "stream-fault-recovery",
        width: 100,
        height: 30,
        capability: TuiColorCapability::Full,
        capability_label: "full",
        private_marker: "PRIVATE_PROVIDER_WIRE_BODY",
        app,
    }
}

fn low_color_fixture() -> IntegratedFixture {
    let mut app = historical_v210_app();
    app.set_banner(banner(false));
    app.push_warning("配置需要检查");
    app.push_error("离线夹具错误可见");
    app.push_agent_event(&AgentEvent::ProviderError {
        provider: "fixture".to_string(),
        status: None,
        classification: "offline".to_string(),
        message: "PRIVATE_MONOCHROME_DETAIL".to_string(),
    });
    app.start_approval(ApprovalRequestView {
        id: None,
        tool_name: "shell".to_string(),
        cwd: "C:/Workspace/云汐".to_string(),
        command: Some("echo safe".to_string()),
        reason: "低色终端也必须保留动作语义".to_string(),
        risk_label: Some("risk: review".to_string()),
    });
    IntegratedFixture {
        name: "low-color-semantics",
        width: 58,
        height: 18,
        capability: TuiColorCapability::Monochrome,
        capability_label: "monochrome",
        private_marker: "PRIVATE_MONOCHROME_DETAIL",
        app,
    }
}

fn render_snapshot(
    app: &YunxiTuiApp,
    width: u16,
    height: u16,
    capability: TuiColorCapability,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| {
            render_tui_frame_with_styles(frame, app, TuiStyleSet::new(capability));
        })
        .expect("fixture draw");
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            let mut row = String::new();
            let mut x = 0;
            while x < width {
                let symbol = buffer[(x, y)].symbol();
                row.push_str(symbol);
                x = x.saturating_add(UnicodeWidthStr::width(symbol).max(1) as u16);
            }
            format!("{y:02}|{}", row.trim_end())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fixture_suite_snapshot() -> String {
    let fixtures = [
        normal_companion_fixture(),
        long_stream_fixture(),
        tool_approval_failure_fixture(),
        history_resize_fixture(),
        stream_fault_fixture(),
        low_color_fixture(),
    ];
    let mut sections = Vec::new();
    for fixture in fixtures {
        let main = render_snapshot(
            &fixture.app,
            fixture.width,
            fixture.height,
            fixture.capability,
        );
        assert!(
            !main.contains(fixture.private_marker),
            "{} leaked private detail in main view",
            fixture.name
        );
        let mut details_app = fixture.app.clone();
        details_app.show_details(None);
        let details = render_snapshot(
            &details_app,
            fixture.width,
            fixture.height,
            fixture.capability,
        );
        assert!(
            details.contains(fixture.private_marker),
            "{} details omitted private diagnostic marker",
            fixture.name
        );
        sections.push(format!(
            "=== {}/main {}x{} style={} ===\n{}\n=== {}/details {}x{} style={} ===\n{}",
            fixture.name,
            fixture.width,
            fixture.height,
            fixture.capability_label,
            main,
            fixture.name,
            fixture.width,
            fixture.height,
            fixture.capability_label,
            details
        ));
    }
    sections.join("\n\n")
}

#[test]
fn integrated_release_fixture_suite_matches_main_and_details_golden() {
    let snapshot = fixture_suite_snapshot();
    if std::env::var_os("YUNXI_UPDATE_SNAPSHOTS").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("snapshots")
            .join("integrated_release_v210.txt");
        std::fs::write(path, format!("{snapshot}\n")).expect("write v2.1.0 golden");
        return;
    }
    assert_eq!(
        snapshot,
        include_str!("snapshots/integrated_release_v210.txt").trim_end_matches(['\r', '\n'])
    );
}
