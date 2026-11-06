//! SHOW-WINDOW without a screen: only what happens before a window opens.

use forthling::{Canvas, Forth, NOT_PERMITTED, Permission};
use forthling_window::Window;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Canvas);
    f.install(&Window);
    f.interpret("ALSO WINDOW").unwrap();
    f
}

#[test]
fn needs_a_canvas() {
    let e = forth().interpret("SHOW-WINDOW").unwrap_err();
    assert_eq!(e.code, -24);
}

#[test]
fn needs_the_permission() {
    let mut f = forth();
    f.interpret("ALSO IMAGE 4 4 CANVAS").unwrap();
    f.permit(Permission::System, false);
    assert_eq!(f.interpret("SHOW-WINDOW").unwrap_err().code, NOT_PERMITTED);
}

#[test]
fn environment_knows_it() {
    let mut f = forth();
    f.interpret("S\" FORTHLING-WINDOW\" ENVIRONMENT? .").unwrap();
    assert_eq!(f.output(), b"-1 ");
}
