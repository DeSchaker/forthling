//! forthling-window: forthling with the canvas words and `SHOW-WINDOW` installed.

fn main() {
    forthling::cli::main(&[
        &forthling::Fixed,
        &forthling::Canvas,
        &forthling::OperatingSystem,
        &forthling_window::Window,
    ]);
}
