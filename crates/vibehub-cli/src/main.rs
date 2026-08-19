fn main() {
    match vibehub_adapters::dispatcher::dispatch(
        std::env::args().skip(1),
        env!("CARGO_PKG_VERSION"),
    ) {
        vibehub_adapters::dispatcher::DispatchOutcome::Handled => {}
        vibehub_adapters::dispatcher::DispatchOutcome::NotHandled => {
            eprintln!(
                "No/bad VibeHub command. Next step (pick one):\n  \
                 vibehub help                                 full command list\n  \
                 vibehub next-action <project>                recommended next action (JSON)\n  \
                 vibehub mcp-status <project>                 show MCP wiring per harness\n  \
                 vibehub mcp-install <project> claude codex opencode   wire MCP into harnesses"
            );
            std::process::exit(2);
        }
    }
}
