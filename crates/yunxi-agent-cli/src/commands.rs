#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InteractiveCommand {
    Exit,
    Help,
    Clear,
    Cwd,
    Session,
    Status,
    Tools,
    Mcp,
    Cost,
    Model(Option<String>),
    Provider(Option<String>),
    Resume(String),
    Debug(Option<String>),
    Details(Option<usize>),
    Controls(Option<String>),
    Companion(Option<String>),
    Unknown(String),
}

pub(crate) fn parse_interactive_command(input: &str) -> Option<InteractiveCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or_default();
    let rest = parts.next().map(str::trim).filter(|value| !value.is_empty());

    Some(match command {
        "/exit" | "/quit" => InteractiveCommand::Exit,
        "/help" => InteractiveCommand::Help,
        "/clear" => InteractiveCommand::Clear,
        "/cwd" => InteractiveCommand::Cwd,
        "/session" => InteractiveCommand::Session,
        "/status" => InteractiveCommand::Status,
        "/tools" => InteractiveCommand::Tools,
        "/mcp" => InteractiveCommand::Mcp,
        "/cost" => InteractiveCommand::Cost,
        "/model" => InteractiveCommand::Model(rest.map(ToOwned::to_owned)),
        "/provider" => InteractiveCommand::Provider(rest.map(ToOwned::to_owned)),
        "/debug" => InteractiveCommand::Debug(rest.map(ToOwned::to_owned)),
        "/details" => InteractiveCommand::Details(match rest {
            Some(value) => match value.parse::<usize>() {
                Ok(id) => Some(id),
                Err(_) => {
                    return Some(InteractiveCommand::Unknown(
                        "/details expects a numeric debug id".to_string(),
                    ));
                }
            },
            None => None,
        }),
        "/controls" => InteractiveCommand::Controls(rest.map(ToOwned::to_owned)),
        "/companion" => InteractiveCommand::Companion(rest.map(ToOwned::to_owned)),
        "/resume" => match rest {
            Some(session_id) => InteractiveCommand::Resume(session_id.to_string()),
            None => InteractiveCommand::Unknown("/resume requires a session id".to_string()),
        },
        other => InteractiveCommand::Unknown(format!("unknown command: {other}")),
    })
}

pub(crate) fn help_text() -> &'static str {
    "Commands:\n\
     /help                 Show this help\n\
     /session              Show active session details\n\
     /status               Show provider, event, tool, and MCP status\n\
     /tools                List fixed and workspace dynamic tools\n\
     /mcp                  Show workspace MCP configuration\n\
     /cost                 Show last-turn and session token usage\n\
     /resume <session_id>  Continue from a saved session\n\
     /model [name]         Show or switch the model field\n\
     /provider [name]      Show or switch the provider field\n\
     /debug events on|off  Show or hide TUI raw debug event summaries\n\
     /details [id]         Show the latest or selected TUI debug detail\n\
     /controls [action]    Show or update the unified control panel\n\
     /companion [on|off]   Show or persist the local companion switch\n\
     /controls clear <scope> asks for explicit confirmation\n\
     /cwd                  Show the active working directory\n\
     /clear                Clear the terminal\n\
     /exit, /quit          Leave YunXi interactive mode"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shared_control_and_companion_commands() {
        assert_eq!(
            parse_interactive_command("/controls clear memory"),
            Some(InteractiveCommand::Controls(Some(
                "clear memory".to_string()
            )))
        );
        assert_eq!(
            parse_interactive_command("/companion off"),
            Some(InteractiveCommand::Companion(Some("off".to_string())))
        );
        assert!(help_text().contains("explicit confirmation"));
    }
}
