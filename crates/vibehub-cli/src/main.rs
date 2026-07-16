fn main() {
    match vibehub_adapters::dispatcher::dispatch(
        std::env::args().skip(1),
        env!("CARGO_PKG_VERSION"),
    ) {
        vibehub_adapters::dispatcher::DispatchOutcome::Handled => {}
        vibehub_adapters::dispatcher::DispatchOutcome::NotHandled => {
            vibehub_adapters::dispatcher::print_help();
            std::process::exit(2);
        }
    }
}
