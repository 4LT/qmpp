use crate::{QMPP_Hook_init, QMPP_Hook_process};

#[derive(Copy, Clone)]
struct QmppRegisterCall {
    pub name_len: usize,
    pub name_ptr: *const u8,
}

static mut QMPP_REGISTER_CALL: Option<QmppRegisterCall> = None;

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn QMPP_register(name_len: usize, name_ptr: *const u8) {
    unsafe {
        QMPP_REGISTER_CALL = Some(QmppRegisterCall { name_len, name_ptr });
    }
}

#[test]
fn init() {
    let expected_name: &str = "hello";

    QMPP_Hook_init();

    let actual_call = unsafe { QMPP_REGISTER_CALL.unwrap() };

    assert_eq!(actual_call.name_len, expected_name.len());
}
